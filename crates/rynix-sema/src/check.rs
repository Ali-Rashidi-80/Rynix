//! Two-pass name resolution and type checking.

#![allow(clippy::too_many_lines)] // single structured checker pass
#![allow(clippy::match_same_arms)] // place-expr stubs share empty bodies

use rustc_hash::{FxHashMap, FxHashSet};
use rynix_ast::{
    AssignOp, BinaryOp, Expr, Item, LiteralKind, Module, Path, Stmt, StructLitExpr, Type, UnaryOp,
};
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
    /// Struct field byte/slot index (i64 slots) for lowering.
    pub field_offsets: FxHashMap<(DefId, Symbol), u32>,
    /// Nullary enum variant `DefId` → discriminant (Phase 17-C).
    pub variant_disc: FxHashMap<DefId, i64>,
    /// Inferred effect sets per function (`DefId`), when source was provided.
    pub fn_effects: FxHashMap<DefId, crate::effects::EffectSet>,
}

/// Run name resolution + type checking. Diagnostics are appended to `sink`.
pub fn analyze(module: &Module<'_>, interner: &mut Interner, sink: &mut dyn DiagSink) -> Analysis {
    analyze_with_source(module, interner, sink, None, 0)
}

/// Like [`analyze`], and when `source` is set run `#^ effect: pure` checks.
pub fn analyze_with_source(
    module: &Module<'_>,
    interner: &mut Interner,
    sink: &mut dyn DiagSink,
    source: Option<&str>,
    source_base: u32,
) -> Analysis {
    let mut cx = Checker::new(interner, sink);
    cx.collect_builtins();
    cx.collect_items(module);
    cx.check_module(module);
    if let Some(src) = source {
        cx.fn_effects = crate::effects::check_module_effects(
            module,
            src,
            source_base,
            cx.interner,
            &cx.defs,
            &cx.path_resolution,
            &cx.fn_sigs,
            cx.sink,
        );
    }
    cx.finish()
}

struct MovedInfo {
    to: Symbol,
    at: Span,
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
    field_offsets: FxHashMap<(DefId, Symbol), u32>,
    /// Enum `DefId` → variant name → (Variant `DefId`, optional payload type).
    enum_variants: FxHashMap<DefId, FxHashMap<Symbol, (DefId, Option<TypeId>)>>,
    /// Nullary variant `DefId` → discriminant i64.
    variant_disc: FxHashMap<DefId, i64>,
    /// Fn `DefId` → function type.
    fn_sigs: FxHashMap<DefId, TypeId>,
    /// Type alias `DefId` → aliased type.
    aliases: FxHashMap<DefId, TypeId>,
    /// Locals/params of linear types that have been moved.
    ownership: FxHashMap<DefId, MovedInfo>,
    /// Filled by [`analyze_with_source`] effect pass.
    fn_effects: FxHashMap<DefId, crate::effects::EffectSet>,
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
            field_offsets: FxHashMap::default(),
            enum_variants: FxHashMap::default(),
            variant_disc: FxHashMap::default(),
            fn_sigs: FxHashMap::default(),
            aliases: FxHashMap::default(),
            ownership: FxHashMap::default(),
            fn_effects: FxHashMap::default(),
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
            field_offsets: self.field_offsets,
            variant_disc: self.variant_disc,
            fn_effects: self.fn_effects,
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
        self.soft_fn("print_i64", vec![self.types.ty_int], self.types.ty_unit);
        self.soft_fn("opaque_i64", vec![self.types.ty_int], self.types.ty_int);

        // Soft std / runtime prelude (Phase 9+).
        self.soft_fn("sleep_ms", vec![self.types.ty_int], self.types.ty_unit);
        self.soft_fn("yield", vec![], self.types.ty_unit);
        self.soft_fn("now_ms", vec![], self.types.ty_int);
        self.soft_fn("fiber_run", vec![], self.types.ty_unit);
        // `tensor` / `signal` / `agent` are reserved keywords — not soft callables (RYX2013).

        // Region Vec/Map (i64 monomorphized) — soft std surface.
        let unit = self.types.ty_unit;
        let i = self.types.ty_int;
        let v = self.types.ty_vec;
        let m = self.types.ty_map;
        self.soft_fn("vec_new", vec![i], v);
        self.soft_fn("vec_push", vec![v, i], unit);
        self.soft_fn("vec_get", vec![v, i], i);
        self.soft_fn("vec_len", vec![v], i);
        self.soft_fn("map_new", vec![i], m);
        self.soft_fn("map_insert", vec![m, i, i], unit);
        self.soft_fn("map_get", vec![m, i], i);
        self.soft_fn("map_len", vec![m], i);

        let vec_sym = self.interner.intern("Vec");
        let vec_def = self.alloc_def(DefKind::BuiltinType { name: vec_sym });
        self.scopes.define(self.module_scope, vec_sym, vec_def);
        self.def_types.insert(vec_def, v);
        let map_sym = self.interner.intern("Map");
        let map_def = self.alloc_def(DefKind::BuiltinType { name: map_sym });
        self.scopes.define(self.module_scope, map_sym, map_def);
        self.def_types.insert(map_def, m);

        // TCP soft surface (recv/send take ptr buffers — soft args are i64 slots).
        let s = self.types.ty_str;
        self.soft_fn("tcp_listen", vec![i], i);
        self.soft_fn("tcp_accept", vec![i], i);
        self.soft_fn("tcp_connect", vec![s, i], i);
        self.soft_fn("tcp_recv", vec![i, i, i], i);
        self.soft_fn("tcp_send", vec![i, i, i], i);
        self.soft_fn("tcp_close", vec![i], unit);

        // JSON / HTTP soft std (v0.1).
        let s = self.types.ty_str;
        self.soft_fn("json_get_i64", vec![s, s], i);
        self.soft_fn("json_has_i64", vec![s, s], i);
        self.soft_fn("http_get_json_i64", vec![s, i, s, s], i);
        self.soft_fn("http_post_json_i64", vec![s, i, s, s, s], i);
        self.soft_fn("http_serve_once_json_i64", vec![i, s, i], i);
        self.soft_fn("http_serve_once_echo_json_i64", vec![i, s, s], i);
        self.soft_fn("http_serve_loop_json_i64", vec![i, s, i, i], i);
        self.soft_fn(
            "http_serve_loop_2paths_json_i64",
            vec![i, s, i, s, i, i],
            i,
        );
        self.soft_fn(
            "http_serve_loop_3paths_json_i64",
            vec![i, s, i, s, i, s, i, i],
            i,
        );
        self.soft_fn("http_serve_loop_path_param_json_i64", vec![i, s, i], i);
        self.soft_fn("http_serve_loop_header_json_i64", vec![i, s, s, i], i);
        self.soft_fn("http_serve_loop_post_echo_json_i64", vec![i, s, s, i, i], i);
        self.soft_fn("http_serve_loop_keepalive_json_i64", vec![i, s, i, i], i);
        self.soft_fn("http_tls_serve_once_json_i64", vec![i, s, i], i);
        self.soft_fn("http_tls_get_json_i64", vec![s, i, s, s], i);
        self.soft_fn("frame_serve_once_echo", vec![i], i);
        self.soft_fn("frame_client_echo", vec![s, i, s], i);
        self.soft_fn("tls_serve_once_echo", vec![i], i);
        self.soft_fn("tls_client_echo", vec![s, i, s], i);

        // Crypto + string-key KV (EndCrypto / EndKV class; evidence via KAT smoke).
        let p = self.types.ty_ptr;
        self.soft_fn("sha256_first_i64", vec![s], i);
        self.soft_fn("hmac_sha256_first_i64", vec![s, s], i);
        self.soft_fn("aes128_gcm_nist_empty_tag_first_i64", vec![], i);
        self.soft_fn("ws_accept_key_eq", vec![s, s], i);
        self.soft_fn("ws_accept_sha1_first_i64", vec![s], i);
        self.soft_fn("ws_frame_roundtrip_ok", vec![], i);
        self.soft_fn("ws_serve_once_echo", vec![i], i);
        self.soft_fn("ws_client_echo", vec![s, i, s], i);
        self.soft_fn("kv_new", vec![i], p);
        self.soft_fn("kv_put", vec![p, s, i], unit);
        self.soft_fn("kv_get", vec![p, s], i);
        self.soft_fn("kv_len", vec![p], i);
        // Portable path filesystem (fopen-backed).
        self.soft_fn("fs_write_file", vec![s, s], i);
        self.soft_fn("fs_read_file", vec![s], s);
        self.soft_fn("fs_read_file_eq", vec![s, s], i);
        self.soft_fn("fs_exists", vec![s], i);
        self.soft_fn("fs_remove_file", vec![s], i);
    }

    fn soft_fn(&mut self, name: &str, params: Vec<TypeId>, ret: TypeId) {
        let sym = self.interner.intern(name);
        let def = self.alloc_def(DefKind::Import {
            name: sym,
            span: Span::empty(0),
        });
        self.scopes.define(self.module_scope, sym, def);
        let ty = self.types.fn_type(params, ret);
        self.def_types.insert(def, ty);
        self.fn_sigs.insert(def, ty);
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
            for (i, field) in s.fields.iter().enumerate() {
                let ty = self.lower_type(field.ty, self.module_scope);
                fields.insert(field.name.name, ty);
                self.field_offsets
                    .insert((def, field.name.name), i as u32);
            }
            self.struct_fields.insert(def, fields);
        }
        for (def, e) in pending_enums {
            let mut variants = FxHashMap::default();
            let mut disc: i64 = 0;
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
                    None => {
                        self.variant_disc.insert(vdef, disc);
                        disc += 1;
                        enum_ty
                    }
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
            Type::App { path, args, span } => {
                let name = path.segments.last().map_or_else(
                    || "?".into(),
                    |s| self.interner.resolve(s.name).to_string(),
                );
                for a in *args {
                    let _ = self.lower_type(a, scope);
                }
                match name.as_str() {
                    "Vec" if args.len() == 1 => {
                        let elem = self.lower_type(args[0], scope);
                        if !self.types.compatible(elem, self.types.ty_int)
                            && !matches!(self.types.kind(elem), TypeKind::Error)
                        {
                            self.sink.emit(errors::type_mismatch(
                                *span,
                                "Vec[i64]",
                                &format!("Vec[{}]", self.display_ty(elem)),
                            ));
                        }
                        self.types.ty_vec
                    }
                    "Map" if args.len() == 2 => {
                        let k = self.lower_type(args[0], scope);
                        let v = self.lower_type(args[1], scope);
                        if (!self.types.compatible(k, self.types.ty_int)
                            || !self.types.compatible(v, self.types.ty_int))
                            && !matches!(self.types.kind(k), TypeKind::Error)
                            && !matches!(self.types.kind(v), TypeKind::Error)
                        {
                            self.sink.emit(errors::type_mismatch(
                                *span,
                                "Map[i64, i64]",
                                &format!(
                                    "Map[{}, {}]",
                                    self.display_ty(k),
                                    self.display_ty(v)
                                ),
                            ));
                        }
                        self.types.ty_map
                    }
                    _ => {
                        self.sink.emit(errors::unresolved_name(
                            *span,
                            &format!("{name}[...]"),
                        ));
                        self.types.ty_error
                    }
                }
            }
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
                let to = self.interner.resolve(l.name.name).to_string();
                self.note_move_from(l.init, &to, l.name.span);
            }
            Stmt::Assign(a) => {
                self.check_assign_target(a.target, scope);
                let target_ty = if a.op == AssignOp::Eq {
                    self.place_ty_reinit(a.target, scope)
                } else {
                    self.check_expr(a.target, scope, None)
                };
                let value_ty = self.check_expr(a.value, scope, Some(target_ty));
                if a.op == AssignOp::Eq {
                    if !self.types.compatible(target_ty, value_ty) {
                        self.sink.emit(errors::type_mismatch(
                            a.value.span(),
                            &self.display_ty(target_ty),
                            &self.display_ty(value_ty),
                        ));
                    }
                    self.note_move_from(a.value, "<assign>", a.value.span());
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
            Stmt::Region(r) => {
                let region_scope = self.scopes.alloc(Some(scope), ScopeKind::Block);
                for s in r.body {
                    self.check_stmt(s, region_scope, expected_ret, saw_return);
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
            Stmt::Match(m) => {
                let scrut = self.check_expr(m.scrutinee, scope, None);
                for arm in m.arms {
                    match &arm.pattern {
                        rynix_ast::MatchPat::Wildcard(_) => {}
                        rynix_ast::MatchPat::Literal(e) => {
                            let pty = self.check_expr(e, scope, Some(scrut));
                            if !self.types.compatible(scrut, pty)
                                && !matches!(self.types.kind(scrut), TypeKind::Error)
                                && !matches!(self.types.kind(pty), TypeKind::Error)
                            {
                                self.sink.emit(errors::type_mismatch(
                                    e.span(),
                                    &self.display_ty(scrut),
                                    &self.display_ty(pty),
                                ));
                            }
                        }
                    }
                    let body_scope = self.scopes.alloc(Some(scope), ScopeKind::Block);
                    for s in arm.body {
                        self.check_stmt(s, body_scope, expected_ret, saw_return);
                    }
                }
                if let Some(body) = m.else_body {
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
            Expr::Field(f) => {
                // Field store (Wave 3): require a mut root binding.
                self.check_field_assign_mut(f.base, scope);
            }
            Expr::Index(i) => {
                // Index store (Phase 17-B): require a mut root binding.
                self.check_field_assign_mut(i.base, scope);
            }
            _ => {}
        }
    }

    fn check_field_assign_mut(&mut self, expr: &Expr<'_>, scope: ScopeId) {
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
                    if let Some(def) = self.scopes.lookup(scope, seg.name) {
                        let kind = &self.defs[def.index() as usize];
                        if matches!(kind, DefKind::Local { .. } | DefKind::Param { .. })
                            && !kind.is_mutable()
                        {
                            self.sink.emit(errors::immutable_assign(
                                p.span,
                                self.interner.resolve(seg.name),
                            ));
                        }
                    }
                }
            }
            Expr::Field(f) => self.check_field_assign_mut(f.base, scope),
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
            Expr::Binary(b) if b.op == BinaryOp::Pipe => {
                let ty = self.check_pipe(b, scope);
                self.node_types.insert(b.id, ty);
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
                if let Expr::Path(p) = c.callee
                    && p.segments.len() == 1
                {
                    let name = self.interner.resolve(p.segments[0].name);
                    if matches!(name, "tensor" | "signal" | "agent") {
                        self.sink.emit(errors::stub_reserved(c.span, name));
                        for arg in c.args {
                            let _ = self.check_expr(arg, scope, None);
                        }
                        self.node_types.insert(c.id, self.types.ty_error);
                        return self.types.ty_error;
                    }
                }
                let callee = self.check_expr(c.callee, scope, None);
                let ty = self.check_call(callee, c.args, c.span, scope);
                self.node_types.insert(c.id, ty);
                ty
            }
            Expr::MethodCall(m) => {
                let recv = self.check_expr(m.receiver, scope, None);
                for a in m.args {
                    let _ = self.check_expr(a, scope, None);
                    self.note_move_from(a, "<arg>", a.span());
                }
                let method = self.interner.resolve(m.method.name).to_string();
                // `import util` then `util.fn(...)` — package-qualified call.
                if matches!(self.types.kind(recv), TypeKind::Module)
                    && let Some(fdef) = self.scopes.lookup(self.module_scope, m.method.name)
                {
                    let fty = self
                        .def_types
                        .get(&fdef)
                        .copied()
                        .unwrap_or(self.types.ty_error);
                    if let TypeKind::Fn { params, ret } = self.types.kind(fty).clone() {
                        if params.len() != m.args.len() {
                            self.sink
                                .emit(errors::wrong_arity(m.span, params.len(), m.args.len()));
                        }
                        self.node_types.insert(m.id, ret);
                        return ret;
                    }
                }
                let ty = match (self.types.kind(recv), method.as_str()) {
                    (TypeKind::Slice(_), "len") => self.types.ty_int,
                    (TypeKind::Vec, "len" | "get") => self.types.ty_int,
                    (TypeKind::Vec, "push") => self.types.ty_unit,
                    (TypeKind::Map, "len" | "get") => self.types.ty_int,
                    (TypeKind::Map, "insert") => self.types.ty_unit,
                    (TypeKind::Module, _) => expected.unwrap_or(self.types.ty_error),
                    (TypeKind::Vec | TypeKind::Map | TypeKind::Slice(_), _) => {
                        self.sink.emit(errors::unknown_method(
                            m.method.span,
                            &self.display_ty(recv),
                            &method,
                        ));
                        self.types.ty_error
                    }
                    _ => {
                        // Soft free function of the same name (receiver = first arg).
                        let sym = m.method.name;
                        if let Some(ret) = self
                            .scopes
                            .lookup(self.module_scope, sym)
                            .and_then(|d| self.def_types.get(&d).copied())
                            .and_then(|t| match self.types.kind(t) {
                                TypeKind::Fn { ret, .. } => Some(*ret),
                                _ => None,
                            })
                        {
                            ret
                        } else {
                            self.sink.emit(errors::unknown_method(
                                m.method.span,
                                &self.display_ty(recv),
                                &method,
                            ));
                            expected.unwrap_or(self.types.ty_error)
                        }
                    }
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
            Expr::StructLit(s) => {
                let ty = self.check_struct_lit(s, scope, expected);
                self.node_types.insert(s.id, ty);
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

    fn check_struct_lit(
        &mut self,
        lit: &StructLitExpr<'_>,
        scope: ScopeId,
        _expected: Option<TypeId>,
    ) -> TypeId {
        if lit.path.segments.is_empty() {
            return self.types.ty_error;
        }
        let first = &lit.path.segments[0];
        let Some(def) = self.scopes.lookup(scope, first.name) else {
            self.sink.emit(errors::unresolved_name(
                first.span,
                self.interner.resolve(first.name),
            ));
            return self.types.ty_error;
        };
        self.path_resolution.insert(lit.path.id, def);
        if !matches!(self.defs[def.index() as usize], DefKind::Struct { .. }) {
            self.sink.emit(errors::type_mismatch(
                lit.path.span,
                "struct type",
                self.interner.resolve(first.name),
            ));
            return self.types.ty_error;
        }
        let struct_ty = self
            .def_types
            .get(&def)
            .copied()
            .unwrap_or(self.types.ty_error);
        let Some(fields) = self.struct_fields.get(&def).cloned() else {
            return self.types.ty_error;
        };

        // Niche-10 / Phase 17: struct literal fields may be `i64` or `str`.
        for (name, &fty) in &fields {
            let ok = self.types.compatible(fty, self.types.ty_int)
                || self.types.compatible(fty, self.types.ty_str);
            if !ok {
                self.sink.emit(errors::type_mismatch(
                    lit.span,
                    "i64 or str",
                    &format!(
                        "field `{}` has type `{}` (struct literals allow i64|str)",
                        self.interner.resolve(*name),
                        self.display_ty(fty)
                    ),
                ));
            }
        }

        let mut seen = FxHashSet::default();
        for init in lit.fields {
            if !seen.insert(init.name.name) {
                self.sink.emit(errors::type_mismatch(
                    init.name.span,
                    "unique field",
                    &format!("duplicate `{}`", self.interner.resolve(init.name.name)),
                ));
            }
            let Some(&fty) = fields.get(&init.name.name) else {
                self.sink.emit(errors::unknown_field(
                    init.name.span,
                    &self.display_ty(struct_ty),
                    self.interner.resolve(init.name.name),
                ));
                let _ = self.check_expr(init.value, scope, None);
                continue;
            };
            let vty = self.check_expr(init.value, scope, Some(fty));
            if !self.types.compatible(fty, vty) {
                self.sink.emit(errors::type_mismatch(
                    init.value.span(),
                    &self.display_ty(fty),
                    &self.display_ty(vty),
                ));
            }
        }
        for (name, _) in &fields {
            if !seen.contains(name) {
                self.sink.emit(errors::type_mismatch(
                    lit.span,
                    &format!("field `{}`", self.interner.resolve(*name)),
                    "missing in struct literal",
                ));
            }
        }
        struct_ty
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
            // Types are not bare values; use `Name { … }` struct literals.
            self.sink.emit(errors::unresolved_name(
                first.span,
                &format!("{} (type used as value)", self.interner.resolve(first.name)),
            ));
            return self.types.ty_error;
        }
        if path.segments.len() == 1 {
            if matches!(
                kind,
                DefKind::Local { .. } | DefKind::Param { .. }
            ) && let Some(moved) = self.ownership.get(&def)
            {
                let name = self.interner.resolve(first.name).to_string();
                let to = self.interner.resolve(moved.to).to_string();
                let at = moved.at;
                self.sink
                    .emit(errors::use_after_move(first.span, &name, &to, at));
            }
            let ty = self
                .def_types
                .get(&def)
                .copied()
                .unwrap_or(self.types.ty_error);
            self.node_types.insert(path.id, ty);
            return ty;
        }
        // Multi-segment: `import util` then `util.fn` resolves `fn` in the
        // flat module scope (unity-compiled package deps, SPEC §6.3–6.4).
        if matches!(
            self.types.kind(
                self.def_types
                    .get(&def)
                    .copied()
                    .unwrap_or(self.types.ty_error)
            ),
            TypeKind::Module
        ) {
            if path.segments.len() == 2 {
                let seg = &path.segments[1];
                if let Some(fdef) = self.scopes.lookup(self.module_scope, seg.name) {
                    let fty = self
                        .def_types
                        .get(&fdef)
                        .copied()
                        .unwrap_or(self.types.ty_error);
                    if matches!(self.types.kind(fty), TypeKind::Fn { .. }) {
                        self.path_resolution.insert(path.id, fdef);
                        self.node_types.insert(path.id, fty);
                        return fty;
                    }
                }
                self.sink.emit(errors::unresolved_name(
                    seg.span,
                    self.interner.resolve(seg.name),
                ));
                return self.types.ty_error;
            }
            return self.types.ty_error; // longer module paths — soft/unknown
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
            | BinaryOp::Percent
            | BinaryOp::Amp
            | BinaryOp::Shr => {
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
            BinaryOp::Pipe => {
                // Handled in check_expr via check_pipe; fallback.
                self.types.ty_error
            }
        }
    }

    fn check_pipe(&mut self, b: &rynix_ast::BinaryExpr<'_>, scope: ScopeId) -> TypeId {
        let _lhs_ty = self.check_expr(b.lhs, scope, None);
        match b.rhs {
            Expr::Path(_) => {
                let callee = self.check_expr(b.rhs, scope, None);
                self.check_call(callee, std::slice::from_ref(&b.lhs), b.span, scope)
            }
            Expr::Call(c) => {
                let callee = self.check_expr(c.callee, scope, None);
                let mut args: Vec<&Expr<'_>> = Vec::with_capacity(1 + c.args.len());
                args.push(b.lhs);
                args.extend(c.args.iter().copied());
                self.check_call(callee, &args, b.span, scope)
            }
            _ => {
                let _ = self.check_expr(b.rhs, scope, None);
                self.sink.emit(errors::type_mismatch(
                    b.span,
                    "path or call on the right of |>",
                    "other expression",
                ));
                self.types.ty_error
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
                    self.note_move_from(arg, "<arg>", arg.span());
                }
                ret
            }
            TypeKind::Module | TypeKind::Error => {
                for arg in args {
                    let _ = self.check_expr(arg, scope, None);
                    self.note_move_from(arg, "<arg>", arg.span());
                }
                self.types.ty_error
            }
            _ => {
                self.sink
                    .emit(errors::not_callable(span, &self.display_ty(callee)));
                for arg in args {
                    let _ = self.check_expr(arg, scope, None);
                    self.note_move_from(arg, "<arg>", arg.span());
                }
                self.types.ty_error
            }
        }
    }

    fn is_linear(&self, ty: TypeId) -> bool {
        matches!(
            self.types.kind(ty),
            TypeKind::Vec
                | TypeKind::Map
                | TypeKind::Ptr
                | TypeKind::Slice(_)
                | TypeKind::Struct(_)
            // Nullary enums are i64 discriminants (Phase 17-C) — Copy, not linear.
        )
    }

    /// Assignment place: resolve type and clear prior move (reinitialize).
    fn place_ty_reinit(&mut self, expr: &Expr<'_>, scope: ScopeId) -> TypeId {
        match expr {
            Expr::Path(p) => {
                let ty = self.check_path_no_move(p, scope);
                if let Some(&def) = self.path_resolution.get(&p.id) {
                    self.ownership.remove(&def);
                }
                ty
            }
            _ => self.check_expr(expr, scope, None),
        }
    }

    fn check_path_no_move(&mut self, path: &Path<'_>, scope: ScopeId) -> TypeId {
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
            self.sink.emit(errors::unresolved_name(
                first.span,
                &format!("{} (type used as value)", self.interner.resolve(first.name)),
            ));
            return self.types.ty_error;
        }
        if path.segments.len() == 1 {
            let ty = self
                .def_types
                .get(&def)
                .copied()
                .unwrap_or(self.types.ty_error);
            self.node_types.insert(path.id, ty);
            return ty;
        }
        self.check_path(path, scope)
    }

    fn note_move_from(&mut self, expr: &Expr<'_>, to: &str, at: Span) {
        let Expr::Path(p) = expr else {
            return;
        };
        if p.segments.len() != 1 {
            return;
        }
        let Some(&def) = self.path_resolution.get(&p.id) else {
            return;
        };
        if !matches!(
            self.defs[def.index() as usize],
            DefKind::Local { .. } | DefKind::Param { .. }
        ) {
            return;
        }
        let Some(ty) = self.def_types.get(&def).copied() else {
            return;
        };
        if !self.is_linear(ty) {
            return;
        }
        let to_sym = self.interner.intern(to);
        self.ownership.insert(def, MovedInfo { to: to_sym, at });
    }
}

// literal helpers removed with reserved `tensor` shape check (RYX2013).
