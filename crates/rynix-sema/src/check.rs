//! Two-pass name resolution and type checking.

#![allow(clippy::too_many_lines)] // single structured checker pass
#![allow(clippy::match_same_arms)] // place-expr stubs share empty bodies

use rustc_hash::FxHashMap;
use rynix_ast::{AssignOp, BinaryOp, Expr, Item, LiteralKind, Module, Path, Stmt, Type, UnaryOp};
use rynix_diag::DiagSink;
use rynix_span::{Interner, Span, Symbol};

use crate::def::{DefId, DefKind};
use crate::errors;
use crate::scope::{ScopeId, ScopeKind, ScopeTree};
use crate::ty::{TypeCtx, TypeId, TypeKind};

/// Result of semantic analysis for one module.
pub struct Analysis {
    pub types: TypeCtx,
    pub defs: Vec<DefKind>,
    pub scopes: ScopeTree,
    pub module_scope: ScopeId,
    /// Expression / statement / item node → type (where applicable).
    pub node_types: FxHashMap<rynix_ast::NodeId, TypeId>,
    /// Local/param `DefId` → type.
    pub def_types: FxHashMap<DefId, TypeId>,
    /// Path `NodeId` → resolved `DefId` (last segment).
    pub path_resolution: FxHashMap<rynix_ast::NodeId, DefId>,
}

/// Run name resolution + type checking. Diagnostics are appended to `sink`.
pub fn analyze(module: &Module<'_>, interner: &mut Interner, sink: &mut dyn DiagSink) -> Analysis {
    let mut cx = Checker::new(interner, sink);
    cx.collect_builtins();
    cx.collect_items(module);
    cx.check_module(module);
    cx.finish()
}

struct Checker<'a> {
    interner: &'a mut Interner,
    sink: &'a mut dyn DiagSink,
    types: TypeCtx,
    defs: Vec<DefKind>,
    scopes: ScopeTree,
    module_scope: ScopeId,
    node_types: FxHashMap<rynix_ast::NodeId, TypeId>,
    def_types: FxHashMap<DefId, TypeId>,
    path_resolution: FxHashMap<rynix_ast::NodeId, DefId>,
    /// Struct `DefId` → field name → field type.
    struct_fields: FxHashMap<DefId, FxHashMap<Symbol, TypeId>>,
    /// Enum `DefId` → variant name → (Variant `DefId`, optional payload type).
    enum_variants: FxHashMap<DefId, FxHashMap<Symbol, (DefId, Option<TypeId>)>>,
    /// Fn `DefId` → function type.
    fn_sigs: FxHashMap<DefId, TypeId>,
    /// Type alias `DefId` → aliased type.
    aliases: FxHashMap<DefId, TypeId>,
}

impl<'a> Checker<'a> {
    fn new(interner: &'a mut Interner, sink: &'a mut dyn DiagSink) -> Self {
        let mut scopes = ScopeTree::new();
        let module_scope = scopes.alloc(None, ScopeKind::Module);
        Self {
            interner,
            sink,
            types: TypeCtx::new(),
            defs: Vec::new(),
            scopes,
            module_scope,
            node_types: FxHashMap::default(),
            def_types: FxHashMap::default(),
            path_resolution: FxHashMap::default(),
            struct_fields: FxHashMap::default(),
            enum_variants: FxHashMap::default(),
            fn_sigs: FxHashMap::default(),
            aliases: FxHashMap::default(),
        }
    }

    fn finish(self) -> Analysis {
        Analysis {
            types: self.types,
            defs: self.defs,
            scopes: self.scopes,
            module_scope: self.module_scope,
            node_types: self.node_types,
            def_types: self.def_types,
            path_resolution: self.path_resolution,
        }
    }

    fn alloc_def(&mut self, kind: DefKind) -> DefId {
        let id = DefId::from_index(self.defs.len() as u32);
        self.defs.push(kind);
        id
    }

    fn display_ty(&self, id: TypeId) -> String {
        self.types
            .display(id, &|d| self.defs[d.index() as usize].name(), self.interner)
    }

    fn collect_builtins(&mut self) {
        for name in ["i64", "f64", "bool", "str"] {
            let sym = self.interner.intern(name);
            let def = self.alloc_def(DefKind::BuiltinType { name: sym });
            self.scopes.define(self.module_scope, sym, def);
            let ty = match name {
                "i64" => self.types.ty_int,
                "f64" => self.types.ty_float,
                "bool" => self.types.ty_bool,
                "str" => self.types.ty_str,
                _ => unreachable!(),
            };
            self.def_types.insert(def, ty);
        }

        // Soft std prelude: `print` accepts any single argument.
        let print = self.interner.intern("print");
        let print_def = self.alloc_def(DefKind::Import {
            name: print,
            span: Span::empty(0),
        });
        self.scopes.define(self.module_scope, print, print_def);
        let print_ty = self
            .types
            .fn_type(vec![self.types.ty_error], self.types.ty_unit);
        self.def_types.insert(print_def, print_ty);
        self.fn_sigs.insert(print_def, print_ty);
    }

    #[allow(clippy::too_many_lines)]
    fn collect_items(&mut self, module: &Module<'_>) {
        // Pass 1: allocate defs and bind names (types may still be unresolved).
        let mut pending_fns = Vec::new();
        let mut pending_structs = Vec::new();
        let mut pending_enums = Vec::new();
        let mut pending_aliases = Vec::new();

        for item in module.items {
            match item {
                Item::Fn(f) => {
                    let def = self.define_item(
                        f.name.name,
                        f.name.span,
                        DefKind::Fn {
                            node: f.id,
                            name: f.name.name,
                            span: f.name.span,
                        },
                    );
                    pending_fns.push((def, f));
                }
                Item::Struct(s) => {
                    let def = self.define_item(
                        s.name.name,
                        s.name.span,
                        DefKind::Struct {
                            node: s.id,
                            name: s.name.name,
                            span: s.name.span,
                        },
                    );
                    let ty = self.types.struct_type(def);
                    self.def_types.insert(def, ty);
                    pending_structs.push((def, s));
                }
                Item::Enum(e) => {
                    let def = self.define_item(
                        e.name.name,
                        e.name.span,
                        DefKind::Enum {
                            node: e.id,
                            name: e.name.name,
                            span: e.name.span,
                        },
                    );
                    let ty = self.types.enum_type(def);
                    self.def_types.insert(def, ty);
                    pending_enums.push((def, e));
                }
                Item::TypeAlias(t) => {
                    let def = self.define_item(
                        t.name.name,
                        t.name.span,
                        DefKind::TypeAlias {
                            node: t.id,
                            name: t.name.name,
                            span: t.name.span,
                        },
                    );
                    pending_aliases.push((def, t));
                }
                Item::Import(i) => {
                    let Some(last) = i.path.segments.last() else {
                        continue;
                    };
                    let _ = self.define_item(
                        last.name,
                        last.span,
                        DefKind::Import {
                            name: last.name,
                            span: i.span,
                        },
                    );
                    // Imports are module values.
                    let def = self.scopes.lookup(self.module_scope, last.name).unwrap();
                    self.def_types.insert(def, self.types.ty_module);
                    self.path_resolution.insert(i.path.id, def);
                }
                Item::Error(_) => {}
            }
        }

        // Pass 2: resolve type aliases, struct fields, enum variants, fn sigs.
        for (def, t) in pending_aliases {
            let ty = self.lower_type(t.ty, self.module_scope);
            self.aliases.insert(def, ty);
            self.def_types.insert(def, ty);
        }
        for (def, s) in pending_structs {
            let mut fields = FxHashMap::default();
            for field in s.fields {
                let ty = self.lower_type(field.ty, self.module_scope);
                fields.insert(field.name.name, ty);
            }
            self.struct_fields.insert(def, fields);
        }
        for (def, e) in pending_enums {
            let mut variants = FxHashMap::default();
            for v in e.variants {
                let vdef = self.alloc_def(DefKind::Variant {
                    parent: def,
                    name: v.name.name,
                    span: v.name.span,
                });
                // Variants are also value constructors in the module scope.
                if let Some(prev) = self.scopes.define(self.module_scope, v.name.name, vdef) {
                    self.dup_error(v.name.name, v.name.span, prev);
                }
                let payload = v.payload.map(|p| self.lower_type(p, self.module_scope));
                let enum_ty = self.types.enum_type(def);
                // Nullary variant: value of enum type; payload variant: fn(payload) -> enum
                let vty = match payload {
                    Some(p) => self.types.fn_type(vec![p], enum_ty),
                    None => enum_ty,
                };
                self.def_types.insert(vdef, vty);
                variants.insert(v.name.name, (vdef, payload));
            }
            self.enum_variants.insert(def, variants);
        }
        for (def, f) in pending_fns {
            let params: Vec<TypeId> = f
                .params
                .iter()
                .map(|p| self.lower_type(p.ty, self.module_scope))
                .collect();
            let ret = f.ret.map_or(self.types.ty_unit, |t| {
                self.lower_type(t, self.module_scope)
            });
            let fty = self.types.fn_type(params, ret);
            self.fn_sigs.insert(def, fty);
            self.def_types.insert(def, fty);
        }
    }

    fn define_item(&mut self, name: Symbol, span: Span, kind: DefKind) -> DefId {
        let def = self.alloc_def(kind);
        if let Some(prev) = self.scopes.define(self.module_scope, name, def) {
            self.dup_error(name, span, prev);
        }
        def
    }

    fn dup_error(&mut self, name: Symbol, span: Span, prev: DefId) {
        let prev_span = self.defs[prev.index() as usize].span().unwrap_or(span);
        self.sink.emit(errors::duplicate_def(
            span,
            self.interner.resolve(name),
            prev_span,
        ));
    }

    fn lower_type(&mut self, ty: &Type<'_>, scope: ScopeId) -> TypeId {
        match ty {
            Type::Error(_) => self.types.ty_error,
            Type::Slice(inner, _) => {
                let elem = self.lower_type(inner, scope);
                self.types.slice(elem)
            }
            Type::Path(path) => self.resolve_type_path(path, scope),
        }
    }

    fn resolve_type_path(&mut self, path: &Path<'_>, scope: ScopeId) -> TypeId {
        if path.segments.len() != 1 {
            // Multi-segment type paths are not supported yet (no nested modules).
            let span = path.span;
            let name = path
                .segments
                .last()
                .map_or("?", |s| self.interner.resolve(s.name));
            self.sink
                .emit(errors::unresolved_name(span, &format!("{name} (path)")));
            return self.types.ty_error;
        }
        let seg = &path.segments[0];
        let Some(def) = self.scopes.lookup(scope, seg.name) else {
            self.sink.emit(errors::unresolved_name(
                seg.span,
                self.interner.resolve(seg.name),
            ));
            return self.types.ty_error;
        };
        self.path_resolution.insert(path.id, def);
        let kind = &self.defs[def.index() as usize];
        if !kind.is_type() {
            self.sink.emit(errors::expected_type_name(
                seg.span,
                self.interner.resolve(seg.name),
            ));
            return self.types.ty_error;
        }
        if let Some(&aliased) = self.aliases.get(&def) {
            return aliased;
        }
        self.def_types
            .get(&def)
            .copied()
            .unwrap_or(self.types.ty_error)
    }

    fn check_module(&mut self, module: &Module<'_>) {
        for item in module.items {
            if let Item::Fn(f) = item {
                self.check_fn(f);
            }
        }
    }

    fn check_fn(&mut self, f: &rynix_ast::FnDef<'_>) {
        let Some(def) = self.scopes.lookup(self.module_scope, f.name.name) else {
            return;
        };
        let Some(&fty) = self.fn_sigs.get(&def) else {
            return;
        };
        let TypeKind::Fn { params, ret } = self.types.kind(fty).clone() else {
            return;
        };

        let fn_scope = self.scopes.alloc(Some(self.module_scope), ScopeKind::Fn);
        for (param, &pty) in f.params.iter().zip(&params) {
            let pdef = self.alloc_def(DefKind::Param {
                name: param.name.name,
                span: param.name.span,
                mutable: false,
            });
            if let Some(prev) = self.scopes.define(fn_scope, param.name.name, pdef) {
                self.dup_error(param.name.name, param.name.span, prev);
            }
            self.def_types.insert(pdef, pty);
        }

        let mut saw_return = false;
        for stmt in f.body {
            self.check_stmt(stmt, fn_scope, ret, &mut saw_return);
        }
        // Implicit unit return is fine when ret is unit; otherwise missing return
        // is only a hard error if the body can complete — keep soft for v0.1.
        let _ = saw_return;
    }

    #[allow(clippy::too_many_lines)]
    fn check_stmt(
        &mut self,
        stmt: &Stmt<'_>,
        scope: ScopeId,
        expected_ret: TypeId,
        saw_return: &mut bool,
    ) {
        match stmt {
            Stmt::Error(_) => {}
            Stmt::Let(l) => {
                let annotated = l.ty.map(|t| self.lower_type(t, scope));
                let init_ty = self.check_expr(l.init, scope, annotated);
                let binding_ty = annotated.unwrap_or(init_ty);
                if let Some(ann) = annotated
                    && !self.types.compatible(ann, init_ty)
                {
                    self.sink.emit(errors::type_mismatch(
                        l.init.span(),
                        &self.display_ty(ann),
                        &self.display_ty(init_ty),
                    ));
                }
                let def = self.alloc_def(DefKind::Local {
                    name: l.name.name,
                    span: l.name.span,
                    mutable: l.mutable,
                });
                if let Some(prev) = self.scopes.define(scope, l.name.name, def) {
                    self.dup_error(l.name.name, l.name.span, prev);
                }
                self.def_types.insert(def, binding_ty);
                self.node_types.insert(l.id, binding_ty);
            }
            Stmt::Assign(a) => {
                let target_ty = self.check_expr(a.target, scope, None);
                self.check_assign_target(a.target, scope);
                let value_ty = self.check_expr(a.value, scope, Some(target_ty));
                if a.op == AssignOp::Eq {
                    if !self.types.compatible(target_ty, value_ty) {
                        self.sink.emit(errors::type_mismatch(
                            a.value.span(),
                            &self.display_ty(target_ty),
                            &self.display_ty(value_ty),
                        ));
                    }
                } else {
                    // Compound assign: both sides numeric-ish for v0.1.
                    let _ = (target_ty, value_ty);
                }
            }
            Stmt::Return(r) => {
                *saw_return = true;
                match r.value {
                    Some(v) => {
                        let ty = self.check_expr(v, scope, Some(expected_ret));
                        if !self.types.compatible(expected_ret, ty) {
                            self.sink.emit(errors::type_mismatch(
                                v.span(),
                                &self.display_ty(expected_ret),
                                &self.display_ty(ty),
                            ));
                        }
                    }
                    None => {
                        if !self.types.compatible(expected_ret, self.types.ty_unit) {
                            self.sink.emit(errors::type_mismatch(
                                r.span,
                                &self.display_ty(expected_ret),
                                "()",
                            ));
                        }
                    }
                }
            }
            Stmt::Break(b) => {
                if !self.scopes.in_loop(scope) {
                    self.sink.emit(errors::break_outside_loop(b.span));
                }
            }
            Stmt::Continue(c) => {
                if !self.scopes.in_loop(scope) {
                    self.sink.emit(errors::continue_outside_loop(c.span));
                }
            }
            Stmt::Loop(l) => {
                let loop_scope = self.scopes.alloc(Some(scope), ScopeKind::Loop);
                for s in l.body {
                    self.check_stmt(s, loop_scope, expected_ret, saw_return);
                }
            }
            Stmt::For(f) => {
                let iter_ty = self.check_expr(f.iter, scope, None);
                let elem_ty = match self.types.kind(iter_ty) {
                    TypeKind::Slice(e) => *e,
                    TypeKind::Error => self.types.ty_error,
                    _ => {
                        // Ranges and other iterables: treat as int for `a..b`.
                        if matches!(self.types.kind(iter_ty), TypeKind::Int) {
                            self.types.ty_int
                        } else {
                            self.types.ty_error
                        }
                    }
                };
                let loop_scope = self.scopes.alloc(Some(scope), ScopeKind::Loop);
                let def = self.alloc_def(DefKind::Local {
                    name: f.binder.name,
                    span: f.binder.span,
                    mutable: false,
                });
                self.scopes.define(loop_scope, f.binder.name, def);
                self.def_types.insert(def, elem_ty);
                for s in f.body {
                    self.check_stmt(s, loop_scope, expected_ret, saw_return);
                }
            }
            Stmt::If(i) => {
                for arm in i.arms {
                    let cond = self.check_expr(arm.cond, scope, Some(self.types.ty_bool));
                    if !self.types.compatible(self.types.ty_bool, cond) {
                        self.sink.emit(errors::type_mismatch(
                            arm.cond.span(),
                            "bool",
                            &self.display_ty(cond),
                        ));
                    }
                    let body_scope = self.scopes.alloc(Some(scope), ScopeKind::Block);
                    for s in arm.body {
                        self.check_stmt(s, body_scope, expected_ret, saw_return);
                    }
                }
                if let Some(body) = i.else_body {
                    let body_scope = self.scopes.alloc(Some(scope), ScopeKind::Block);
                    for s in body {
                        self.check_stmt(s, body_scope, expected_ret, saw_return);
                    }
                }
            }
            Stmt::Expr(e) => {
                let _ = self.check_expr(e.expr, scope, None);
            }
        }
    }

    fn check_assign_target(&mut self, expr: &Expr<'_>, scope: ScopeId) {
        match expr {
            Expr::Path(p) => {
                if let Some(&def) = self.path_resolution.get(&p.id) {
                    let kind = &self.defs[def.index() as usize];
                    if matches!(kind, DefKind::Local { .. } | DefKind::Param { .. })
                        && !kind.is_mutable()
                    {
                        self.sink.emit(errors::immutable_assign(
                            p.span,
                            self.interner.resolve(kind.name()),
                        ));
                    }
                } else if let Some(seg) = p.segments.last() {
                    // Try lookup for message quality.
                    if let Some(def) = self.scopes.lookup(scope, seg.name) {
                        let kind = &self.defs[def.index() as usize];
                        if !kind.is_mutable() {
                            self.sink.emit(errors::immutable_assign(
                                p.span,
                                self.interner.resolve(seg.name),
                            ));
                        }
                    }
                }
            }
            Expr::Field(_) | Expr::Index(_) => {
                // Place expressions — mutability of the base is checked lightly.
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_expr(&mut self, expr: &Expr<'_>, scope: ScopeId, expected: Option<TypeId>) -> TypeId {
        match expr {
            Expr::Error(e) => {
                self.node_types.insert(e.id, self.types.ty_error);
                self.types.ty_error
            }
            Expr::Literal(l) => {
                let ty = match l.kind {
                    LiteralKind::Int => self.types.ty_int,
                    LiteralKind::Float => self.types.ty_float,
                    LiteralKind::Str => self.types.ty_str,
                    LiteralKind::True | LiteralKind::False => self.types.ty_bool,
                    LiteralKind::Nil => self.types.ty_nil,
                };
                self.node_types.insert(l.id, ty);
                ty
            }
            Expr::Path(p) => self.check_path(p, scope),
            Expr::Unary(u) => {
                let operand = self.check_expr(u.operand, scope, None);
                let ty = match u.op {
                    UnaryOp::Not => {
                        if !self.types.compatible(self.types.ty_bool, operand) {
                            self.sink.emit(errors::type_mismatch(
                                u.operand.span(),
                                "bool",
                                &self.display_ty(operand),
                            ));
                        }
                        self.types.ty_bool
                    }
                    UnaryOp::Neg => {
                        if !matches!(
                            self.types.kind(operand),
                            TypeKind::Int | TypeKind::Float | TypeKind::Error
                        ) {
                            self.sink.emit(errors::type_mismatch(
                                u.operand.span(),
                                "i64 or f64",
                                &self.display_ty(operand),
                            ));
                        }
                        operand
                    }
                };
                self.node_types.insert(u.id, ty);
                ty
            }
            Expr::Binary(b) => {
                let lhs = self.check_expr(b.lhs, scope, None);
                let rhs = self.check_expr(b.rhs, scope, None);
                let ty = self.check_binary(b.op, lhs, rhs, b.span);
                self.node_types.insert(b.id, ty);
                ty
            }
            Expr::Cast(c) => {
                let _ = self.check_expr(c.expr, scope, None);
                let ty = self.lower_type(c.ty, scope);
                self.node_types.insert(c.id, ty);
                ty
            }
            Expr::Call(c) => {
                let callee = self.check_expr(c.callee, scope, None);
                let ty = self.check_call(callee, c.args, c.span, scope);
                self.node_types.insert(c.id, ty);
                ty
            }
            Expr::MethodCall(m) => {
                // v0.1: treat as opaque if receiver is a module; else error lightly.
                let recv = self.check_expr(m.receiver, scope, None);
                for a in m.args {
                    let _ = self.check_expr(a, scope, None);
                }
                let ty = if matches!(self.types.kind(recv), TypeKind::Module | TypeKind::Error) {
                    expected.unwrap_or(self.types.ty_error)
                } else {
                    self.types.ty_error
                };
                self.node_types.insert(m.id, ty);
                ty
            }
            Expr::Index(i) => {
                let base = self.check_expr(i.base, scope, None);
                let index = self.check_expr(i.index, scope, Some(self.types.ty_int));
                if !self.types.compatible(self.types.ty_int, index) {
                    self.sink.emit(errors::type_mismatch(
                        i.index.span(),
                        "i64",
                        &self.display_ty(index),
                    ));
                }
                let ty = match self.types.kind(base) {
                    TypeKind::Slice(e) => *e,
                    TypeKind::Error => self.types.ty_error,
                    _ => {
                        self.sink.emit(errors::type_mismatch(
                            i.base.span(),
                            "slice",
                            &self.display_ty(base),
                        ));
                        self.types.ty_error
                    }
                };
                self.node_types.insert(i.id, ty);
                ty
            }
            Expr::Field(f) => {
                let base = self.check_expr(f.base, scope, None);
                let ty = match self.types.kind(base) {
                    TypeKind::Struct(def) => {
                        let def = *def;
                        if let Some(fields) = self.struct_fields.get(&def) {
                            if let Some(&fty) = fields.get(&f.field.name) {
                                fty
                            } else {
                                self.sink.emit(errors::unknown_field(
                                    f.field.span,
                                    &self.display_ty(base),
                                    self.interner.resolve(f.field.name),
                                ));
                                self.types.ty_error
                            }
                        } else {
                            self.types.ty_error
                        }
                    }
                    TypeKind::Module | TypeKind::Error => self.types.ty_error,
                    _ => {
                        self.sink.emit(errors::unknown_field(
                            f.field.span,
                            &self.display_ty(base),
                            self.interner.resolve(f.field.name),
                        ));
                        self.types.ty_error
                    }
                };
                self.node_types.insert(f.id, ty);
                ty
            }
            Expr::Array(a) => {
                let mut elem_ty = expected.and_then(|e| match self.types.kind(e) {
                    TypeKind::Slice(inner) => Some(*inner),
                    _ => None,
                });
                for e in a.elems {
                    let t = self.check_expr(e, scope, elem_ty);
                    if elem_ty.is_none() {
                        elem_ty = Some(t);
                    } else if let Some(et) = elem_ty
                        && !self.types.compatible(et, t)
                    {
                        self.sink.emit(errors::type_mismatch(
                            e.span(),
                            &self.display_ty(et),
                            &self.display_ty(t),
                        ));
                    }
                }
                let ty = self.types.slice(elem_ty.unwrap_or(self.types.ty_error));
                self.node_types.insert(a.id, ty);
                ty
            }
            Expr::Spawn(s) => {
                let _ = self.check_expr(s.callee, scope, None);
                // spawn returns unit for now.
                self.node_types.insert(s.id, self.types.ty_unit);
                self.types.ty_unit
            }
        }
    }

    fn check_path(&mut self, path: &Path<'_>, scope: ScopeId) -> TypeId {
        // Only single-segment value paths are resolved in v0.1; `a::b` is treated
        // as module path: resolve `a`, then opaque.
        if path.segments.is_empty() {
            return self.types.ty_error;
        }
        let first = &path.segments[0];
        let Some(def) = self.scopes.lookup(scope, first.name) else {
            self.sink.emit(errors::unresolved_name(
                first.span,
                self.interner.resolve(first.name),
            ));
            return self.types.ty_error;
        };
        self.path_resolution.insert(path.id, def);
        let kind = &self.defs[def.index() as usize];
        if kind.is_type() && !matches!(kind, DefKind::Variant { .. }) {
            // Types are not values (no struct literals in v0.1).
            self.sink.emit(errors::unresolved_name(
                first.span,
                &format!("{} (type used as value)", self.interner.resolve(first.name)),
            ));
            return self.types.ty_error;
        }
        if path.segments.len() == 1 {
            return self
                .def_types
                .get(&def)
                .copied()
                .unwrap_or(self.types.ty_error);
        }
        // Multi-segment: if first is a module, remainder is opaque (external).
        if matches!(
            self.types.kind(
                self.def_types
                    .get(&def)
                    .copied()
                    .unwrap_or(self.types.ty_error)
            ),
            TypeKind::Module
        ) {
            return self.types.ty_error; // unknown external — soft
        }
        // Otherwise unresolved nested path.
        let last = path.segments.last().unwrap();
        self.sink.emit(errors::unresolved_name(
            last.span,
            self.interner.resolve(last.name),
        ));
        self.types.ty_error
    }

    fn check_binary(&mut self, op: BinaryOp, lhs: TypeId, rhs: TypeId, span: Span) -> TypeId {
        match op {
            BinaryOp::Or | BinaryOp::And => {
                for (ty, side) in [(lhs, "left"), (rhs, "right")] {
                    if !self.types.compatible(self.types.ty_bool, ty) {
                        self.sink.emit(errors::type_mismatch(
                            span,
                            "bool",
                            &format!("{}: {}", side, self.display_ty(ty)),
                        ));
                    }
                }
                self.types.ty_bool
            }
            BinaryOp::EqEq
            | BinaryOp::BangEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                if !self.types.compatible(lhs, rhs)
                    && !matches!(self.types.kind(lhs), TypeKind::Error)
                    && !matches!(self.types.kind(rhs), TypeKind::Error)
                {
                    self.sink.emit(errors::type_mismatch(
                        span,
                        &self.display_ty(lhs),
                        &self.display_ty(rhs),
                    ));
                }
                self.types.ty_bool
            }
            BinaryOp::DotDot | BinaryOp::DotDotEq => {
                // Range yields int iterator surface as int for for-loops.
                self.types.ty_int
            }
            BinaryOp::Plus
            | BinaryOp::Minus
            | BinaryOp::Star
            | BinaryOp::Slash
            | BinaryOp::Percent => {
                if self.types.compatible(lhs, rhs) {
                    lhs
                } else if matches!(self.types.kind(lhs), TypeKind::Error) {
                    rhs
                } else if matches!(self.types.kind(rhs), TypeKind::Error) {
                    lhs
                } else {
                    self.sink.emit(errors::type_mismatch(
                        span,
                        &self.display_ty(lhs),
                        &self.display_ty(rhs),
                    ));
                    self.types.ty_error
                }
            }
        }
    }

    fn check_call(
        &mut self,
        callee: TypeId,
        args: &[&Expr<'_>],
        span: Span,
        scope: ScopeId,
    ) -> TypeId {
        match self.types.kind(callee).clone() {
            TypeKind::Fn { params, ret } => {
                if params.len() != args.len() {
                    self.sink
                        .emit(errors::wrong_arity(span, params.len(), args.len()));
                }
                for (i, arg) in args.iter().enumerate() {
                    let expected = params.get(i).copied();
                    let aty = self.check_expr(arg, scope, expected);
                    if let Some(exp) = expected
                        && !self.types.compatible(exp, aty)
                    {
                        self.sink.emit(errors::type_mismatch(
                            arg.span(),
                            &self.display_ty(exp),
                            &self.display_ty(aty),
                        ));
                    }
                }
                ret
            }
            TypeKind::Module | TypeKind::Error => {
                for arg in args {
                    let _ = self.check_expr(arg, scope, None);
                }
                self.types.ty_error
            }
            _ => {
                self.sink
                    .emit(errors::not_callable(span, &self.display_ty(callee)));
                for arg in args {
                    let _ = self.check_expr(arg, scope, None);
                }
                self.types.ty_error
            }
        }
    }
}
