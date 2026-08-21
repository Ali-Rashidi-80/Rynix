//! Lower typed AST + sema analysis into RIR.

#![allow(clippy::too_many_lines)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use rynix_ast::{
    AssignOp, BinaryOp, Expr, Item, LiteralKind, Module as AstModule, Stmt, UnaryOp,
};
use rynix_sema::{Analysis, TypeId, TypeKind};
use rynix_span::{Interner, Symbol};
use rustc_hash::FxHashMap;

use crate::builder::FunctionBuilder;
use crate::ir::{BlockId, CmpOp, FuncId, Inst, IrTy, Module, ValueId};

#[derive(Clone, Copy)]
enum Local {
    /// Mutable / address-taken: alloca pointer.
    Slot(ValueId),
    /// Immutable SSA value (Braun-style direct binding).
    Ssa(ValueId),
}

struct LoopFrame {
    header: BlockId,
    exit: BlockId,
}

/// Lower an entire module. `analysis` must be from the same AST.
/// `src` is the original source (for recovering literal values from spans).
pub fn lower_module(
    ast: &AstModule<'_>,
    analysis: &Analysis,
    interner: &mut Interner,
    src: &str,
    base: u32,
) -> Module {
    let mut module = Module::new();

    // First pass: declare all functions so calls can resolve.
    let mut fn_map: FxHashMap<Symbol, FuncId> = FxHashMap::default();
    for item in ast.items {
        if let Item::Fn(f) = item {
            let id = FuncId(module.funcs.len() as u32);
            // Placeholder; replaced below.
            module
                .funcs
                .push(FunctionBuilder::new(f.name.name, IrTy::Unit).finish());
            module.func_names.push(f.name.name);
            fn_map.insert(f.name.name, id);
        }
    }

    for item in ast.items {
        if let Item::Fn(f) = item {
            let fid = fn_map[&f.name.name];
            let func = lower_function(f, analysis, interner, &fn_map, src, base);
            module.funcs[fid.0 as usize] = func;
        }
    }

    module
}

fn map_ty(analysis: &Analysis, ty: TypeId) -> IrTy {
    match analysis.types.kind(ty) {
        TypeKind::Error | TypeKind::Never | TypeKind::Unit | TypeKind::Nil | TypeKind::Module => {
            IrTy::Unit
        }
        TypeKind::Bool => IrTy::Bool,
        TypeKind::Int => IrTy::I64,
        TypeKind::Float => IrTy::F64,
        TypeKind::Str => IrTy::Str,
        TypeKind::Slice(_) | TypeKind::Struct(_) | TypeKind::Enum(_) | TypeKind::Fn { .. } => {
            IrTy::Ptr
        }
    }
}

fn lower_function(
    f: &rynix_ast::FnDef<'_>,
    analysis: &Analysis,
    interner: &mut Interner,
    fn_map: &FxHashMap<Symbol, FuncId>,
    src: &str,
    base: u32,
) -> crate::ir::Function {
    let ret_ty = analysis
        .scopes
        .lookup(analysis.module_scope, f.name.name)
        .and_then(|d| analysis.def_types.get(&d).copied())
        .map(|fty| match analysis.types.kind(fty) {
            TypeKind::Fn { ret, .. } => map_ty(analysis, *ret),
            _ => IrTy::Unit,
        })
        .unwrap_or(IrTy::Unit);

    let mut b = FunctionBuilder::new(f.name.name, ret_ty);
    let mut locals: FxHashMap<Symbol, Local> = FxHashMap::default();
    let mut loops: Vec<LoopFrame> = Vec::new();

    // Params: create allocas and store the incoming values (uniform addressing).
    for param in f.params {
        let ty = map_ty(
            analysis,
            param_type(analysis, f, param),
        );
        let incoming = b.add_param(ty);
        let slot = b.alloc(ty, param.span);
        b.store(slot, incoming);
        locals.insert(param.name.name, Local::Slot(slot));
    }

    let mut cx = LowerCtx {
        b: &mut b,
        analysis,
        interner,
        fn_map,
        locals: &mut locals,
        loops: &mut loops,
        src,
        base,
    };
    for stmt in f.body {
        cx.stmt(stmt);
    }

    // Implicit return if block not terminated.
    if !cx.is_terminated() {
        if ret_ty == IrTy::Unit {
            cx.b.ret(None);
        } else {
            // Missing return — emit unreachable for verifier friendliness.
            let _ = cx.b.push(Inst::Unreachable);
        }
    }

    b.finish()
}

fn param_type(
    analysis: &Analysis,
    f: &rynix_ast::FnDef<'_>,
    param: &rynix_ast::Param<'_>,
) -> TypeId {
    if let Some(def) = analysis.scopes.lookup(analysis.module_scope, f.name.name)
        && let Some(&fty) = analysis.def_types.get(&def)
        && let TypeKind::Fn { params, .. } = analysis.types.kind(fty)
    {
        // Match by position.
        if let Some(idx) = f.params.iter().position(|p| p.name.name == param.name.name)
            && let Some(&ty) = params.get(idx)
        {
            return ty;
        }
    }
    analysis.types.ty_error
}

struct LowerCtx<'a, 'b> {
    b: &'a mut FunctionBuilder,
    analysis: &'b Analysis,
    interner: &'b mut Interner,
    fn_map: &'b FxHashMap<Symbol, FuncId>,
    locals: &'a mut FxHashMap<Symbol, Local>,
    loops: &'a mut Vec<LoopFrame>,
    src: &'b str,
    base: u32,
}

impl LowerCtx<'_, '_> {
    fn is_terminated(&self) -> bool {
        let block = self.b.func.block(self.b.current());
        block
            .insts
            .last()
            .is_some_and(|id| self.b.func.inst(*id).is_terminator())
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        if self.is_terminated() {
            return;
        }
        match stmt {
            Stmt::Error(_) => {}
            Stmt::Break(_) => {
                if let Some(frame) = self.loops.last() {
                    self.b.jump(frame.exit, vec![]);
                } else {
                    let _ = self.b.push(Inst::Unreachable);
                }
            }
            Stmt::Continue(_) => {
                if let Some(frame) = self.loops.last() {
                    self.b.jump(frame.header, vec![]);
                } else {
                    let _ = self.b.push(Inst::Unreachable);
                }
            }
            Stmt::Let(l) => {
                let init = self.expr(l.init);
                if l.mutable {
                    let ty = self
                        .analysis
                        .node_types
                        .get(&l.id)
                        .map(|t| map_ty(self.analysis, *t))
                        .unwrap_or_else(|| self.b.func.value_ty(init));
                    let slot = self.b.alloc(ty, l.span);
                    self.b.store(slot, init);
                    self.locals.insert(l.name.name, Local::Slot(slot));
                } else {
                    // Immutable → direct SSA binding (Braun-style).
                    self.locals.insert(l.name.name, Local::Ssa(init));
                }
            }
            Stmt::Assign(a) => {
                if let Expr::Path(p) = a.target
                    && let Some(seg) = p.segments.last()
                    && let Some(Local::Slot(slot)) = self.locals.get(&seg.name).copied()
                {
                    let rhs = self.expr(a.value);
                    let val = match a.op {
                        AssignOp::Eq => rhs,
                        AssignOp::PlusEq => {
                            let cur = self.b.load(slot);
                            self.b.push_value(Inst::IAdd(cur, rhs))
                        }
                        AssignOp::MinusEq => {
                            let cur = self.b.load(slot);
                            self.b.push_value(Inst::ISub(cur, rhs))
                        }
                        AssignOp::StarEq => {
                            let cur = self.b.load(slot);
                            self.b.push_value(Inst::IMul(cur, rhs))
                        }
                        AssignOp::SlashEq => {
                            let cur = self.b.load(slot);
                            self.b.push_value(Inst::IDiv(cur, rhs))
                        }
                        AssignOp::PercentEq => {
                            let cur = self.b.load(slot);
                            self.b.push_value(Inst::IRem(cur, rhs))
                        }
                    };
                    self.b.store(slot, val);
                } else {
                    let _ = self.expr(a.value);
                }
            }
            Stmt::Return(r) => {
                let v = r.value.map(|e| self.expr(e));
                self.b.ret(v);
            }
            Stmt::Expr(e) => {
                let _ = self.expr(e.expr);
            }
            Stmt::If(i) => {
                self.lower_if(i);
            }
            Stmt::Loop(l) => {
                let header = self.b.create_block();
                let body = self.b.create_block();
                let exit = self.b.create_block();
                self.b.jump(header, vec![]);
                self.b.switch_to(header);
                self.b.seal_block(header);
                self.b.jump(body, vec![]);
                self.b.switch_to(body);
                self.b.seal_block(body);
                self.loops.push(LoopFrame { header, exit });
                for s in l.body {
                    self.stmt(s);
                }
                self.loops.pop();
                if !self.is_terminated() {
                    self.b.jump(header, vec![]);
                }
                self.b.switch_to(exit);
                self.b.seal_block(exit);
            }
            Stmt::For(f) => {
                let base = self.expr(f.iter);
                let len = self.b.push_value(Inst::ArrayLen(base));
                let i_slot = self.b.alloc(IrTy::I64, f.span);
                let zero = self.b.iconst(0);
                self.b.store(i_slot, zero);
                let binder_slot = self.b.alloc(IrTy::I64, f.span);
                self.locals
                    .insert(f.binder.name, Local::Slot(binder_slot));

                let header = self.b.create_block();
                let body = self.b.create_block();
                let exit = self.b.create_block();
                self.b.jump(header, vec![]);
                self.b.switch_to(header);
                self.b.seal_block(header);
                let i = self.b.load(i_slot);
                let cond = self.b.push_value(Inst::ICmp(CmpOp::Lt, i, len));
                self.b.br(cond, body, vec![], exit, vec![]);
                self.b.switch_to(body);
                self.b.seal_block(body);
                let _ = self.b.push(Inst::BoundsCheck { index: i, len });
                let elem = self.b.push_value(Inst::LoadIndex { base, index: i });
                self.b.store(binder_slot, elem);
                self.loops.push(LoopFrame { header, exit });
                for s in f.body {
                    self.stmt(s);
                }
                self.loops.pop();
                if !self.is_terminated() {
                    let i2 = self.b.load(i_slot);
                    let one = self.b.iconst(1);
                    let next = self.b.push_value(Inst::IAdd(i2, one));
                    self.b.store(i_slot, next);
                    self.b.jump(header, vec![]);
                }
                self.b.switch_to(exit);
                self.b.seal_block(exit);
            }
        }
    }

    fn lower_if(&mut self, i: &rynix_ast::IfStmt<'_>) {
        // Flatten to nested br for the first arm; elif/else chain.
        let join = self.b.create_block();
        self.lower_if_arms(&i.arms, i.else_body, join);
        self.b.switch_to(join);
        self.b.seal_block(join);
    }

    fn lower_if_arms(
        &mut self,
        arms: &[rynix_ast::IfArm<'_>],
        else_body: Option<&[Stmt<'_>]>,
        join: crate::ir::BlockId,
    ) {
        if arms.is_empty() {
            if let Some(body) = else_body {
                for s in body {
                    self.stmt(s);
                }
            }
            if !self.is_terminated() {
                self.b.jump(join, vec![]);
            }
            return;
        }
        let arm = &arms[0];
        let cond = self.expr(arm.cond);
        let then_b = self.b.create_block();
        let else_b = self.b.create_block();
        self.b.br(cond, then_b, vec![], else_b, vec![]);

        self.b.switch_to(then_b);
        self.b.seal_block(then_b);
        for s in arm.body {
            self.stmt(s);
        }
        if !self.is_terminated() {
            self.b.jump(join, vec![]);
        }

        self.b.switch_to(else_b);
        self.b.seal_block(else_b);
        self.lower_if_arms(&arms[1..], else_body, join);
    }

    fn expr(&mut self, expr: &Expr<'_>) -> ValueId {
        match expr {
            Expr::Error(_) => self.b.iconst(0),
            Expr::Literal(l) => {
                let text = self.span_text(l.span);
                match l.kind {
                    LiteralKind::Int => self.b.iconst(parse_int_lit(text).unwrap_or(0)),
                    LiteralKind::Float => {
                        let n = text.replace('_', "").parse().unwrap_or(0.0);
                        self.b.fconst(n)
                    }
                    LiteralKind::True => self.b.bconst(true),
                    LiteralKind::False => self.b.bconst(false),
                    LiteralKind::Str => {
                        let inner = strip_string_lit(text);
                        let sym = self.interner.intern(&inner);
                        self.b.sconst(sym)
                    }
                    LiteralKind::Nil => self.b.push_value(Inst::Nil),
                }
            }
            Expr::Path(p) => {
                if let Some(seg) = p.segments.last()
                    && let Some(local) = self.locals.get(&seg.name).copied()
                {
                    return match local {
                        Local::Slot(slot) => self.b.load(slot),
                        Local::Ssa(v) => v,
                    };
                }
                // Function ref as value not supported — zero.
                self.b.iconst(0)
            }
            Expr::Unary(u) => {
                let x = self.expr(u.operand);
                match u.op {
                    UnaryOp::Neg => {
                        if self.b.func.value_ty(x) == IrTy::F64 {
                            self.b.push_value(Inst::FNeg(x))
                        } else {
                            self.b.push_value(Inst::INeg(x))
                        }
                    }
                    UnaryOp::Not => self.b.push_value(Inst::BNot(x)),
                }
            }
            Expr::Binary(bin) => {
                let l = self.expr(bin.lhs);
                let r = self.expr(bin.rhs);
                self.lower_binary(bin.op, l, r)
            }
            Expr::Cast(c) => {
                // Bitcast-ish: just re-evaluate; types may differ.
                self.expr(c.expr)
            }
            Expr::Call(c) => self.lower_call(c),
            Expr::MethodCall(m) => {
                for a in m.args {
                    let _ = self.expr(a);
                }
                let _ = self.expr(m.receiver);
                self.b.iconst(0)
            }
            Expr::Index(i) => {
                let base = self.expr(i.base);
                let index = self.expr(i.index);
                let len = self.b.push_value(Inst::ArrayLen(base));
                let _ = self.b.push(Inst::BoundsCheck { index, len });
                self.b.push_value(Inst::LoadIndex { base, index })
            }
            Expr::Field(f) => {
                let base = self.expr(f.base);
                // Resolve struct DefId from the base expression's type.
                let offset = self
                    .analysis
                    .node_types
                    .get(&f.base.id())
                    .and_then(|&ty| match self.analysis.types.kind(ty) {
                        TypeKind::Struct(def) => Some(*def),
                        _ => None,
                    })
                    .and_then(|def| {
                        self.analysis
                            .field_offsets
                            .get(&(def, f.field.name))
                            .copied()
                    })
                    .unwrap_or(0);
                let idx = self.b.iconst(i64::from(offset));
                let slot = self.b.push_value(Inst::GepI64 {
                    base,
                    index: idx,
                });
                self.b.load(slot)
            }
            Expr::Array(a) => {
                // Layout: [len | e0 | e1 | …] as contiguous i64 slots via heap_alloc.
                let n = i64::try_from(a.elems.len()).unwrap_or(0);
                let bytes = self.b.iconst((n + 1) * 8);
                let alloc_name = self.interner.intern("rynix_rt_heap_alloc");
                let base = self.b.call_ext(alloc_name, vec![bytes], IrTy::Ptr);
                let zero = self.b.iconst(0);
                let len_slot = self.b.push_value(Inst::GepI64 {
                    base,
                    index: zero,
                });
                let len_v = self.b.iconst(n);
                self.b.store(len_slot, len_v);
                for (i, e) in a.elems.iter().enumerate() {
                    let val = self.expr(e);
                    let idx = self.b.iconst(i64::try_from(i).unwrap_or(0) + 1);
                    let slot = self.b.push_value(Inst::GepI64 { base, index: idx });
                    self.b.store(slot, val);
                }
                base
            }
            Expr::Spawn(s) => {
                // spawn <call-or-path>: schedule fiber; runtime takes fn ptr + null arg.
                let _ = self.expr(s.callee);
                let spawn = self.interner.intern("rynix_rt_spawn");
                let nil = self.b.push_value(Inst::Nil);
                // Pass null fn for v0 if we cannot materialize a function pointer from
                // a path; real fn-pointer emission is codegen's job for named callees.
                if let Expr::Path(p) = s.callee
                    && p.segments.len() == 1
                {
                    let name = p.segments[0].name;
                    // Encode as CallExt with the callee name as a second convention:
                    // codegen maps rynix_rt_spawn + named symbol.
                    let tag = self.interner.intern("__spawn_fn");
                    let marker = self.b.sconst(name);
                    let _ = tag;
                    self.b.call_ext(spawn, vec![marker, nil], IrTy::Ptr)
                } else if let Expr::Call(c) = s.callee
                    && let Expr::Path(p) = c.callee
                    && p.segments.len() == 1
                {
                    // Evaluate args for side effects, then spawn the function.
                    for a in c.args {
                        let _ = self.expr(a);
                    }
                    let name = p.segments[0].name;
                    let marker = self.b.sconst(name);
                    self.b.call_ext(spawn, vec![marker, nil], IrTy::Ptr)
                } else {
                    self.b.call_ext(spawn, vec![nil, nil], IrTy::Ptr)
                }
            }
        }
    }

    fn lower_binary(&mut self, op: BinaryOp, l: ValueId, r: ValueId) -> ValueId {
        let floaty = self.b.func.value_ty(l) == IrTy::F64 || self.b.func.value_ty(r) == IrTy::F64;
        match op {
            BinaryOp::Plus => {
                if floaty {
                    self.b.push_value(Inst::FAdd(l, r))
                } else {
                    self.b.push_value(Inst::IAdd(l, r))
                }
            }
            BinaryOp::Minus => {
                if floaty {
                    self.b.push_value(Inst::FSub(l, r))
                } else {
                    self.b.push_value(Inst::ISub(l, r))
                }
            }
            BinaryOp::Star => {
                if floaty {
                    self.b.push_value(Inst::FMul(l, r))
                } else {
                    self.b.push_value(Inst::IMul(l, r))
                }
            }
            BinaryOp::Slash => {
                if floaty {
                    self.b.push_value(Inst::FDiv(l, r))
                } else {
                    self.b.push_value(Inst::IDiv(l, r))
                }
            }
            BinaryOp::Percent => self.b.push_value(Inst::IRem(l, r)),
            BinaryOp::EqEq => self.cmp(CmpOp::Eq, l, r, floaty),
            BinaryOp::BangEq => self.cmp(CmpOp::Ne, l, r, floaty),
            BinaryOp::Lt => self.cmp(CmpOp::Lt, l, r, floaty),
            BinaryOp::LtEq => self.cmp(CmpOp::Le, l, r, floaty),
            BinaryOp::Gt => self.cmp(CmpOp::Gt, l, r, floaty),
            BinaryOp::GtEq => self.cmp(CmpOp::Ge, l, r, floaty),
            BinaryOp::And => self.b.push_value(Inst::IMul(l, r)),
            BinaryOp::Or => self.b.push_value(Inst::IAdd(l, r)),
            BinaryOp::DotDot | BinaryOp::DotDotEq => l,
        }
    }

    fn cmp(&mut self, op: CmpOp, l: ValueId, r: ValueId, floaty: bool) -> ValueId {
        if floaty {
            self.b.push_value(Inst::FCmp(op, l, r))
        } else {
            self.b.push_value(Inst::ICmp(op, l, r))
        }
    }

    fn lower_call(&mut self, c: &rynix_ast::CallExpr<'_>) -> ValueId {
        let mut args = Vec::new();
        for a in c.args {
            args.push(self.expr(a));
        }
        if let Expr::Path(p) = c.callee
            && p.segments.len() == 1
        {
            let name = p.segments[0].name;
            if let Some(&fid) = self.fn_map.get(&name) {
                let ret = self
                    .analysis
                    .scopes
                    .lookup(self.analysis.module_scope, name)
                    .and_then(|d| self.analysis.def_types.get(&d).copied())
                    .map(|t| match self.analysis.types.kind(t) {
                        TypeKind::Fn { ret, .. } => map_ty(self.analysis, *ret),
                        _ => IrTy::Unit,
                    })
                    .unwrap_or(IrTy::Unit);
                return self.b.call(fid, args, ret);
            }
            // External / builtin.
            let n = self.interner.resolve(name);
            if n == "tensor" && args.len() == 2 {
                return args[1];
            }
            let (ext_name, ret) = match n {
                "sleep_ms" => (self.interner.intern("rynix_rt_sleep_ms"), IrTy::Unit),
                "yield" => (self.interner.intern("rynix_rt_yield"), IrTy::Unit),
                "now_ms" => (self.interner.intern("rynix_rt_now_ms"), IrTy::I64),
                "fiber_run" => (self.interner.intern("rynix_rt_run"), IrTy::Unit),
                "vec_new" => (self.interner.intern("rynix_rt_vec_i64_new"), IrTy::Ptr),
                "vec_push" => (self.interner.intern("rynix_rt_vec_i64_push"), IrTy::Unit),
                "vec_get" => (self.interner.intern("rynix_rt_vec_i64_get"), IrTy::I64),
                "vec_len" => (self.interner.intern("rynix_rt_vec_i64_len"), IrTy::I64),
                "map_new" => (self.interner.intern("rynix_rt_map_i64_new"), IrTy::Ptr),
                "map_insert" => (self.interner.intern("rynix_rt_map_i64_insert"), IrTy::Unit),
                "map_get" => (self.interner.intern("rynix_rt_map_i64_get"), IrTy::I64),
                "map_len" => (self.interner.intern("rynix_rt_map_i64_len"), IrTy::I64),
                "tcp_listen" => (self.interner.intern("rynix_rt_tcp_listen"), IrTy::I64),
                "tcp_accept" => (self.interner.intern("rynix_rt_tcp_accept"), IrTy::I64),
                "tcp_close" => (self.interner.intern("rynix_rt_tcp_close"), IrTy::Unit),
                "signal" | "agent" => (name, IrTy::Unit),
                _ => {
                    let ret = self
                        .analysis
                        .node_types
                        .get(&c.id)
                        .map(|t| map_ty(self.analysis, *t))
                        .unwrap_or(IrTy::Unit);
                    (name, ret)
                }
            };
            return self.b.call_ext(ext_name, args, ret);
        }
        let _ = self.expr(c.callee);
        self.b.iconst(0)
    }

    fn span_text(&self, span: rynix_span::Span) -> &str {
        let lo = (span.lo().saturating_sub(self.base)) as usize;
        let hi = (span.hi().saturating_sub(self.base)) as usize;
        let hi = hi.min(self.src.len());
        let lo = lo.min(hi);
        &self.src[lo..hi]
    }
}

fn parse_int_lit(text: &str) -> Option<i64> {
    let t = text.replace('_', "");
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i64::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        i64::from_str_radix(rest, 8).ok()
    } else if let Some(rest) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        i64::from_str_radix(rest, 2).ok()
    } else {
        t.parse().ok()
    }
}

fn strip_string_lit(text: &str) -> String {
    let t = text.strip_prefix('"').unwrap_or(text);
    let t = t.strip_suffix('"').unwrap_or(t);
    // Minimal unescape for common sequences.
    t.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}
