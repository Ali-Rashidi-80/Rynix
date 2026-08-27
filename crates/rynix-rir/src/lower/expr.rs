impl LowerCtx<'_, '_> {
    /// `i * j + i` / `i + i * j` → `i * (j + 1)` (nested-style hot path).
    fn try_lower_i_mul_plus_i(&mut self, lhs: &Expr<'_>, rhs: &Expr<'_>) -> Option<ValueId> {
        self.fold_i_mul_plus_i(lhs, rhs)
            .or_else(|| self.fold_i_mul_plus_i(rhs, lhs))
    }

    fn fold_i_mul_plus_i(&mut self, mul: &Expr<'_>, same: &Expr<'_>) -> Option<ValueId> {
        let Expr::Binary(b) = mul else {
            return None;
        };
        if b.op != BinaryOp::Star {
            return None;
        }
        let (i_expr, j_expr) = if paths_equal(&b.lhs, same) {
            (b.lhs, b.rhs)
        } else if paths_equal(&b.rhs, same) {
            (b.rhs, b.lhs)
        } else {
            return None;
        };
        let i = self.expr(i_expr);
        let j = self.expr(j_expr);
        let one = self.b.iconst(1);
        let jp1 = self.b.push_value(Inst::IAdd(j, one));
        Some(self.lower_int_mul(i, jp1))
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
                        Local::Slot(slot) => self.load_slot(slot),
                        Local::MutSsa(_) => self.load_sym(seg.name),
                        Local::Ssa(v) => v,
                    };
                }
                // Nullary enum variant → discriminant i64, or boxed disc for payload enums.
                if let Some(&def) = self.analysis.path_resolution.get(&p.id)
                    && let Some(&disc) = self.analysis.variant_disc.get(&def)
                {
                    if self.variant_parent_has_payload(def) {
                        let d = self.b.iconst(disc);
                        let z = self.b.iconst(0);
                        let ext = self.interner.intern("rynix_rt_enum_box_i64");
                        return self.b.call_ext(ext, vec![d, z], IrTy::Ptr);
                    }
                    return self.b.iconst(disc);
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
                if bin.op == BinaryOp::Pipe {
                    return self.lower_pipe(bin);
                }
                if bin.op == BinaryOp::Plus {
                    if let Some(v) = self.try_lower_i_mul_plus_i(bin.lhs, bin.rhs) {
                        return v;
                    }
                }
                if bin.op == BinaryOp::Percent {
                    let l = self.expr(bin.lhs);
                    let r = self.expr(bin.rhs);
                    return self.lower_int_rem(l, r);
                }
                let l = self.expr(bin.lhs);
                let r = self.expr(bin.rhs);
                self.lower_binary(bin.op, l, r)
            }
            Expr::Cast(c) => {
                // Bitcast-ish: just re-evaluate; types may differ.
                self.expr(c.expr)
            }
            Expr::Call(c) => self.lower_call(c),
            Expr::MethodCall(m) => self.lower_method_call(m),
            Expr::Index(i) => {
                let base = self.expr(i.base);
                let index = self.expr(i.index);
                let len = self.b.push_value(Inst::ArrayLen(base));
                let _ = self.b.push(Inst::BoundsCheck { index, len });
                self.b.push_value(Inst::LoadIndex { base, index })
            }
            Expr::Field(f) => {
                let slot = self.lower_field_slot(f);
                let ty = self
                    .analysis
                    .node_types
                    .get(&f.id)
                    .copied()
                    .map(|t| map_ty(self.analysis, t))
                    .unwrap_or(IrTy::I64);
                self.b.load_as(slot, ty)
            }
            Expr::StructLit(s) => {
                let def = self
                    .analysis
                    .path_resolution
                    .get(&s.path.id)
                    .copied()
                    .or_else(|| {
                        self.analysis.node_types.get(&s.id).and_then(|&ty| {
                            match self.analysis.types.kind(ty) {
                                TypeKind::Struct(d) => Some(*d),
                                _ => None,
                            }
                        })
                    });
                let nslots = def
                    .map(|d| {
                        self.analysis
                            .field_offsets
                            .iter()
                            .filter(|((sd, _), _)| *sd == d)
                            .map(|(_, off)| *off + 1)
                            .max()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                let n = i64::from(nslots);
                let bytes = self.b.iconst(n * 8);
                let alloc_name = self.interner.intern("rynix_rt_heap_alloc");
                let base = self.b.call_ext(alloc_name, vec![bytes], IrTy::Ptr);
                for init in s.fields {
                    let offset = def
                        .and_then(|d| {
                            self.analysis
                                .field_offsets
                                .get(&(d, init.name.name))
                                .copied()
                        })
                        .unwrap_or(0);
                    let val = self.expr(init.value);
                    let idx = self.b.iconst(i64::from(offset));
                    let slot = self.b.push_value(Inst::GepI64 {
                        base,
                        index: idx,
                    });
                    self.b.store(slot, val);
                }
                base
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
                    self.lower_int_mul(l, r)
                }
            }
            BinaryOp::Slash => {
                if floaty {
                    self.b.push_value(Inst::FDiv(l, r))
                } else {
                    self.lower_int_div(l, r)
                }
            }
            BinaryOp::Percent => self.lower_int_rem(l, r),
            BinaryOp::Amp => self.b.push_value(Inst::IAnd(l, r)),
            BinaryOp::Shr => self.b.push_value(Inst::LShr(l, r)),
            BinaryOp::EqEq => self.cmp(CmpOp::Eq, l, r, floaty),
            BinaryOp::BangEq => self.cmp(CmpOp::Ne, l, r, floaty),
            BinaryOp::Lt => self.cmp(CmpOp::Lt, l, r, floaty),
            BinaryOp::LtEq => self.cmp(CmpOp::Le, l, r, floaty),
            BinaryOp::Gt => self.cmp(CmpOp::Gt, l, r, floaty),
            BinaryOp::GtEq => self.cmp(CmpOp::Ge, l, r, floaty),
            BinaryOp::And => self.b.push_value(Inst::BAnd(l, r)),
            BinaryOp::Or => self.b.push_value(Inst::BOr(l, r)),
            BinaryOp::DotDot | BinaryOp::DotDotEq => l,
            BinaryOp::Pipe => l,
        }
    }

    fn lower_pipe(&mut self, bin: &rynix_ast::BinaryExpr<'_>) -> ValueId {
        let mut args = vec![self.expr(bin.lhs)];
        let (name, call_id) = match bin.rhs {
            Expr::Path(p) if p.segments.len() == 1 => (p.segments[0].name, bin.id),
            Expr::Call(c) => {
                for a in c.args {
                    args.push(self.expr(a));
                }
                if let Expr::Path(p) = c.callee
                    && p.segments.len() == 1
                {
                    (p.segments[0].name, c.id)
                } else {
                    let _ = self.expr(c.callee);
                    return self.b.iconst(0);
                }
            }
            _ => {
                let _ = self.expr(bin.rhs);
                return self.b.iconst(0);
            }
        };
        if self.interner.resolve(name) == "popcount" && args.len() == 1 {
            return self.b.push_value(Inst::CtPop(args[0]));
        }
        if let Some(&fid) = self.fn_map.get(&name) {
            let ret = self
                .analysis
                .scopes
                .lookup(self.analysis.module_scope, name)
                .and_then(|d| self.analysis.def_types.get(&d).copied())
                .map(|ty| match self.analysis.types.kind(ty) {
                    TypeKind::Fn { ret, .. } => map_ty(self.analysis, *ret),
                    _ => IrTy::Unit,
                })
                .unwrap_or(IrTy::Unit);
            if let Some(fdef) = self.fn_bodies.get(&name)
                && is_inlineable(fdef, self.fn_map)
            {
                return self.inline_call(fdef, &args, ret);
            }
            return self.b.call(fid, args, ret);
        }
        let n = self.interner.resolve(name).to_string();
        self.lower_soft_call(&n, name, args, call_id)
    }

    fn cmp(&mut self, op: CmpOp, l: ValueId, r: ValueId, floaty: bool) -> ValueId {
        if floaty {
            self.b.push_value(Inst::FCmp(op, l, r))
        } else {
            self.b.push_value(Inst::ICmp(op, l, r))
        }
    }

    /// Expand Euclidean `gcd` to binary GCD (Stein) using `@llvm.cttz` — same
    /// result for non-negative inputs, typically far fewer `%` ops.
    fn lower_binary_gcd(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let z = self.b.iconst(0);
        let ret = self.b.create_block();
        let ret_v = self.b.append_block_param(ret, IrTy::I64);
        let u_zero = self.b.create_block();
        let check_v = self.b.create_block();
        let v_zero = self.b.create_block();
        let start = self.b.create_block();
        let loop_h = self.b.create_block();
        let after_ctz = self.b.create_block();
        let do_swap = self.b.create_block();
        let no_swap = self.b.create_block();
        let cont = self.b.create_block();
        let done = self.b.create_block();

        let a0 = self.b.push_value(Inst::ICmp(CmpOp::Eq, a, z));
        self.b.br(a0, u_zero, vec![], check_v, vec![]);

        self.b.switch_to(u_zero);
        self.b.jump(ret, vec![b]);
        self.b.seal_block(u_zero);

        self.b.switch_to(check_v);
        self.b.seal_block(check_v);
        let b0 = self.b.push_value(Inst::ICmp(CmpOp::Eq, b, z));
        self.b.br(b0, v_zero, vec![], start, vec![]);

        self.b.switch_to(v_zero);
        self.b.jump(ret, vec![a]);
        self.b.seal_block(v_zero);

        self.b.switch_to(start);
        self.b.seal_block(start);
        let uv = self.b.push_value(Inst::IOr(a, b));
        let shift = self.b.push_value(Inst::Cttz(uv));
        let tzu = self.b.push_value(Inst::Cttz(a));
        let u1 = self.b.push_value(Inst::LShr(a, tzu));
        self.b.jump(loop_h, vec![u1, b]);

        let u_phi = self.b.append_block_param(loop_h, IrTy::I64);
        let v_phi = self.b.append_block_param(loop_h, IrTy::I64);
        self.b.switch_to(loop_h);
        self.b.seal_block(loop_h);
        let tzv = self.b.push_value(Inst::Cttz(v_phi));
        let v1 = self.b.push_value(Inst::LShr(v_phi, tzv));
        self.b.jump(after_ctz, vec![u_phi, v1]);

        let u2 = self.b.append_block_param(after_ctz, IrTy::I64);
        let v2 = self.b.append_block_param(after_ctz, IrTy::I64);
        self.b.switch_to(after_ctz);
        self.b.seal_block(after_ctz);
        let gt = self.b.push_value(Inst::ICmp(CmpOp::Gt, u2, v2));
        self.b.br(gt, do_swap, vec![], no_swap, vec![]);

        self.b.switch_to(do_swap);
        self.b.jump(cont, vec![v2, u2]);
        self.b.seal_block(do_swap);

        self.b.switch_to(no_swap);
        self.b.jump(cont, vec![u2, v2]);
        self.b.seal_block(no_swap);

        let u3 = self.b.append_block_param(cont, IrTy::I64);
        let v3 = self.b.append_block_param(cont, IrTy::I64);
        self.b.switch_to(cont);
        self.b.seal_block(cont);
        let v4 = self.b.push_value(Inst::ISub(v3, u3));
        let vnz = self.b.push_value(Inst::ICmp(CmpOp::Ne, v4, z));
        self.b.br(vnz, loop_h, vec![u3, v4], done, vec![u3]);

        let u_done = self.b.append_block_param(done, IrTy::I64);
        self.b.switch_to(done);
        self.b.seal_block(done);
        let res = self.b.push_value(Inst::LShl(u_done, shift));
        self.b.jump(ret, vec![res]);

        self.b.switch_to(ret);
        self.b.seal_block(ret);
        ret_v
    }

    /// Expand a small leaf callee in-place (early `return` joins at `inline_merge`).
    fn inline_call(&mut self, f: &FnDef<'_>, args: &[ValueId], ret: IrTy) -> ValueId {
        let snapshot = self.locals.clone();
        let nonneg_snapshot = self.mut_nonneg_syms.clone();
        let positive_snapshot = self.mut_positive_syms.clone();
        let excl_snapshot = self.mut_excl_bound.clone();
        let binding_snapshot = self.mut_binding_sites.clone();
        let loop_depth = self.loops.len();
        let carried_depth = self.loop_carried.len();
        let carried_linear_depth = self.loop_carried_linear.len();

        for (param, &arg) in f.params.iter().zip(args.iter()) {
            self.locals.insert(param.name.name, Local::Ssa(arg));
            if self.value_is_nonneg(arg) {
                self.mut_nonneg_syms.insert(param.name.name);
            }
            if self.value_is_strictly_positive(arg) {
                self.mut_positive_syms.insert(param.name.name);
            }
            if let Some(b) = self.value_excl_bound(arg) {
                self.mut_excl_bound.insert(param.name.name, b);
            }
        }

        let merge = self.b.create_block();
        let ret_param = if ret == IrTy::Unit {
            None
        } else {
            Some(self.b.append_block_param(merge, ret))
        };
        self.inlining = true;
        self.inline_ret = None;
        self.inline_merge = Some(merge);
        for stmt in f.body {
            self.stmt(stmt);
        }
        if !self.is_terminated() {
            if ret_param.is_some() {
                let z = self.b.iconst(0);
                self.b.jump(merge, vec![z]);
            } else {
                self.b.jump(merge, vec![]);
            }
        }
        self.inlining = false;
        self.inline_merge = None;
        let _ = self.inline_ret.take();

        self.b.switch_to(merge);
        self.b.seal_block(merge);

        *self.locals = snapshot;
        *self.mut_nonneg_syms = nonneg_snapshot;
        *self.mut_positive_syms = positive_snapshot;
        *self.mut_excl_bound = excl_snapshot;
        *self.mut_binding_sites = binding_snapshot;
        self.loops.truncate(loop_depth);
        self.loop_carried.truncate(carried_depth);
        self.loop_carried_linear.truncate(carried_linear_depth);
        ret_param.unwrap_or_else(|| self.b.iconst(0))
    }

    fn lower_call(&mut self, c: &rynix_ast::CallExpr<'_>) -> ValueId {
        let mut args = Vec::new();
        for a in c.args {
            args.push(self.expr(a));
        }
        if let Expr::Path(p) = c.callee {
            // Payload enum ctor: Some(x) → box(disc, x)
            if let Some(&def) = self.analysis.path_resolution.get(&p.id)
                && let Some(&disc) = self.analysis.variant_disc.get(&def)
                && self.analysis.variant_payload.contains_key(&def)
                && args.len() == 1
            {
                let d = self.b.iconst(disc);
                let pty = self.analysis.variant_payload[&def];
                let ext = if matches!(self.analysis.types.kind(pty), TypeKind::Str) {
                    self.interner.intern("rynix_rt_enum_box_str")
                } else {
                    self.interner.intern("rynix_rt_enum_box_i64")
                };
                return self.b.call_ext(ext, vec![d, args[0]], IrTy::Ptr);
            }
            // `fn(...)` or `pkg.fn(...)` after `import pkg` (flat unity symbols).
            let name = if p.segments.len() == 1 {
                Some(p.segments[0].name)
            } else if p.segments.len() == 2 {
                Some(p.segments[1].name)
            } else {
                None
            };
            if let Some(name) = name {
            if self.interner.resolve(name) == "popcount" && args.len() == 1 {
                return self.b.push_value(Inst::CtPop(args[0]));
            }
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
                let euclid = self
                    .fn_bodies
                    .get(&name)
                    .is_some_and(|fdef| is_euclidean_gcd_fn(fdef));
                if euclid && args.len() == 2 {
                    return self.lower_binary_gcd(args[0], args[1]);
                }
                if let Some(fdef) = self.fn_bodies.get(&name)
                    && is_inlineable(fdef, self.fn_map)
                {
                    return self.inline_call(fdef, &args, ret);
                }
                return self.b.call(fid, args, ret);
            }
            // External / builtin (single-segment only).
            if p.segments.len() == 1 {
                let n = self.interner.resolve(name).to_string();
                if n == "tensor" && args.len() == 2 {
                    return args[1];
                }
                return self.lower_soft_call(&n, name, args, c.id);
            }
            }
        }
        let _ = self.expr(c.callee);
        self.b.iconst(0)
    }

    fn lower_method_call(&mut self, m: &rynix_ast::MethodCallExpr<'_>) -> ValueId {
        let recv_ty = self.analysis.node_types.get(&m.receiver.id()).copied();
        let kind = recv_ty.map(|t| self.analysis.types.kind(t));
        // `import util` then `util.fn(...)` — flat package call (no receiver arg).
        if kind.is_some_and(|k| matches!(k, TypeKind::Module)) {
            let name = m.method.name;
            let mut args = Vec::new();
            for a in m.args {
                args.push(self.expr(a));
            }
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
                if let Some(fdef) = self.fn_bodies.get(&name)
                    && is_inlineable(fdef, self.fn_map)
                {
                    return self.inline_call(fdef, &args, ret);
                }
                return self.b.call(fid, args, ret);
            }
            return self.b.iconst(0);
        }
        let recv = self.expr(m.receiver);
        let method = self.interner.resolve(m.method.name).to_string();
        if method == "len" && kind.is_some_and(|k| matches!(k, TypeKind::Slice(_))) {
            return self.b.push_value(Inst::ArrayLen(recv));
        }
        let mut args = vec![recv];
        for a in m.args {
            args.push(self.expr(a));
        }
        let soft = match (method.as_str(), kind) {
            ("insert", Some(TypeKind::Map)) => "map_insert",
            ("insert", Some(TypeKind::MapStrI64)) => "map_str_i64_insert",
            ("insert", Some(TypeKind::MapStrStr)) => "map_str_str_insert",
            ("push", Some(TypeKind::Vec)) => "vec_push",
            ("push", Some(TypeKind::VecStr)) => "vec_str_push",
            ("push", Some(TypeKind::VecBool)) => "vec_bool_push",
            ("get", Some(TypeKind::Map)) => "map_get",
            ("get", Some(TypeKind::MapStrI64)) => "map_str_i64_get",
            ("get", Some(TypeKind::MapStrStr)) => "map_str_str_get",
            ("get", Some(TypeKind::Vec)) => "vec_get",
            ("get", Some(TypeKind::VecStr)) => "vec_str_get",
            ("get", Some(TypeKind::VecBool)) => "vec_bool_get",
            ("len", Some(TypeKind::Map)) => "map_len",
            ("len", Some(TypeKind::MapStrI64)) => "map_str_i64_len",
            ("len", Some(TypeKind::MapStrStr)) => "map_str_str_len",
            ("len", Some(TypeKind::Vec)) => "vec_len",
            ("len", Some(TypeKind::VecStr)) => "vec_str_len",
            ("len", Some(TypeKind::VecBool)) => "vec_bool_len",
            (other, _) => other,
        };
        let soft = soft.to_string();
        self.lower_soft_call(&soft, m.method.name, args, m.id)
    }

    fn lower_soft_call(
        &mut self,
        n: &str,
        name: Symbol,
        args: Vec<ValueId>,
        node: rynix_ast::NodeId,
    ) -> ValueId {
        let (ext_name, ret) = match n {
            "sleep_ms" => (self.interner.intern("rynix_rt_sleep_ms"), IrTy::Unit),
            "yield" => (self.interner.intern("rynix_rt_yield"), IrTy::Unit),
            "now_ms" => (self.interner.intern("rynix_rt_now_ms"), IrTy::I64),
            "print_i64" => (self.interner.intern("rynix_rt_print_i64"), IrTy::Unit),
            "opaque_i64" => (self.interner.intern("rynix_rt_opaque_i64"), IrTy::I64),
            "fiber_run" => (self.interner.intern("rynix_rt_run"), IrTy::Unit),
            "vec_new" => (self.interner.intern("rynix_rt_vec_i64_new"), IrTy::Ptr),
            "vec_push" => (self.interner.intern("rynix_rt_vec_i64_push"), IrTy::Unit),
            "vec_get" => (self.interner.intern("rynix_rt_vec_i64_get"), IrTy::I64),
            "vec_len" => (self.interner.intern("rynix_rt_vec_i64_len"), IrTy::I64),
            "vec_str_new" => (self.interner.intern("rynix_rt_vec_str_new"), IrTy::Ptr),
            "vec_str_push" => (self.interner.intern("rynix_rt_vec_str_push"), IrTy::Unit),
            "vec_str_get" => (self.interner.intern("rynix_rt_vec_str_get"), IrTy::Str),
            "vec_str_len" => (self.interner.intern("rynix_rt_vec_str_len"), IrTy::I64),
            "vec_bool_new" => (self.interner.intern("rynix_rt_vec_bool_new"), IrTy::Ptr),
            "vec_bool_push" => (self.interner.intern("rynix_rt_vec_bool_push"), IrTy::Unit),
            "vec_bool_get" => (self.interner.intern("rynix_rt_vec_bool_get"), IrTy::Bool),
            "vec_bool_len" => (self.interner.intern("rynix_rt_vec_bool_len"), IrTy::I64),
            "map_new" => (self.interner.intern("rynix_rt_map_i64_new"), IrTy::Ptr),
            "map_insert" => (self.interner.intern("rynix_rt_map_i64_insert"), IrTy::Unit),
            "map_get" => (self.interner.intern("rynix_rt_map_i64_get"), IrTy::I64),
            "map_len" => (self.interner.intern("rynix_rt_map_i64_len"), IrTy::I64),
            "map_str_i64_new" => (self.interner.intern("rynix_rt_map_str_i64_new"), IrTy::Ptr),
            "map_str_i64_insert" => {
                (self.interner.intern("rynix_rt_map_str_i64_insert"), IrTy::Unit)
            }
            "map_str_i64_get" => (self.interner.intern("rynix_rt_map_str_i64_get"), IrTy::I64),
            "map_str_i64_len" => (self.interner.intern("rynix_rt_map_str_i64_len"), IrTy::I64),
            "map_str_str_new" => (self.interner.intern("rynix_rt_map_str_str_new"), IrTy::Ptr),
            "map_str_str_insert" => {
                (self.interner.intern("rynix_rt_map_str_str_insert"), IrTy::Unit)
            }
            "map_str_str_get" => (self.interner.intern("rynix_rt_map_str_str_get"), IrTy::Str),
            "map_str_str_len" => (self.interner.intern("rynix_rt_map_str_str_len"), IrTy::I64),
            "tcp_listen" => (self.interner.intern("rynix_rt_tcp_listen"), IrTy::I64),
            "tcp_accept" => (self.interner.intern("rynix_rt_tcp_accept"), IrTy::I64),
            "tcp_connect" => (self.interner.intern("rynix_rt_tcp_connect"), IrTy::I64),
            "tcp_recv" => (self.interner.intern("rynix_rt_tcp_recv"), IrTy::I64),
            "tcp_send" => (self.interner.intern("rynix_rt_tcp_send"), IrTy::I64),
            "tcp_close" => (self.interner.intern("rynix_rt_tcp_close"), IrTy::Unit),
            "json_get_i64" => (self.interner.intern("rynix_rt_json_get_i64"), IrTy::I64),
            "json_has_i64" => (self.interner.intern("rynix_rt_json_has_i64"), IrTy::I64),
            "http_get_json_i64" => (self.interner.intern("rynix_rt_http_get_json_i64"), IrTy::I64),
            "http_post_json_i64" => (self.interner.intern("rynix_rt_http_post_json_i64"), IrTy::I64),
            "http_serve_once_json_i64" => {
                (self.interner.intern("rynix_rt_http_serve_once_json_i64"), IrTy::I64)
            }
            "http_serve_once_echo_json_i64" => {
                (self.interner.intern("rynix_rt_http_serve_once_echo_json_i64"), IrTy::I64)
            }
            "http_serve_loop_json_i64" => {
                (self.interner.intern("rynix_rt_http_serve_loop_json_i64"), IrTy::I64)
            }
            "http_serve_loop_2paths_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_2paths_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_3paths_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_3paths_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_path_param_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_path_param_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_header_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_header_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_bearer_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_bearer_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_post_echo_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_post_echo_json_i64"),
                    IrTy::I64,
                )
            }
            "http_serve_loop_keepalive_json_i64" => {
                (
                    self.interner.intern("rynix_rt_http_serve_loop_keepalive_json_i64"),
                    IrTy::I64,
                )
            }
            "http_tls_serve_once_json_i64" => {
                (self.interner.intern("rynix_rt_http_tls_serve_once_json_i64"), IrTy::I64)
            }
            "http_tls_get_json_i64" => {
                (self.interner.intern("rynix_rt_http_tls_get_json_i64"), IrTy::I64)
            }
            "frame_serve_once_echo" => {
                (self.interner.intern("rynix_rt_frame_serve_once_echo"), IrTy::I64)
            }
            "frame_client_echo" => (self.interner.intern("rynix_rt_frame_client_echo"), IrTy::I64),
            "tls_serve_once_echo" => {
                (self.interner.intern("rynix_rt_tls_serve_once_echo"), IrTy::I64)
            }
            "tls_client_echo" => (self.interner.intern("rynix_rt_tls_client_echo"), IrTy::I64),
            "sha256_first_i64" => (self.interner.intern("rynix_rt_sha256_first_i64"), IrTy::I64),
            "hmac_sha256_first_i64" => {
                (self.interner.intern("rynix_rt_hmac_sha256_first_i64"), IrTy::I64)
            }
            "aes128_gcm_nist_empty_tag_first_i64" => (
                self.interner
                    .intern("rynix_rt_aes128_gcm_nist_empty_tag_first_i64"),
                IrTy::I64,
            ),
            "ws_accept_key_eq" => (self.interner.intern("rynix_rt_ws_accept_key_eq"), IrTy::I64),
            "ws_accept_sha1_first_i64" => {
                (self.interner.intern("rynix_rt_ws_accept_sha1_first_i64"), IrTy::I64)
            }
            "ws_frame_roundtrip_ok" => {
                (self.interner.intern("rynix_rt_ws_frame_roundtrip_ok"), IrTy::I64)
            }
            "ws_serve_once_echo" => {
                (self.interner.intern("rynix_rt_ws_serve_once_echo"), IrTy::I64)
            }
            "ws_client_echo" => (self.interner.intern("rynix_rt_ws_client_echo"), IrTy::I64),
            "kv_new" => (self.interner.intern("rynix_rt_kv_new"), IrTy::Ptr),
            "kv_put" => (self.interner.intern("rynix_rt_kv_put"), IrTy::Unit),
            "kv_get" => (self.interner.intern("rynix_rt_kv_get"), IrTy::I64),
            "kv_len" => (self.interner.intern("rynix_rt_kv_len"), IrTy::I64),
            "fs_write_file" => (self.interner.intern("rynix_rt_fs_write_file"), IrTy::I64),
            "fs_read_file" => (self.interner.intern("rynix_rt_fs_read_file"), IrTy::Str),
            "fs_read_file_eq" => (self.interner.intern("rynix_rt_fs_read_file_eq"), IrTy::I64),
            "fs_exists" => (self.interner.intern("rynix_rt_fs_exists"), IrTy::I64),
            "fs_remove_file" => (self.interner.intern("rynix_rt_fs_remove_file"), IrTy::I64),
            "signal" | "agent" => (name, IrTy::Unit),
            _ => {
                let ret = self
                    .analysis
                    .node_types
                    .get(&node)
                    .map(|t| map_ty(self.analysis, *t))
                    .unwrap_or(IrTy::Unit);
                (name, ret)
            }
        };
        self.b.call_ext(ext_name, args, ret)
    }

    fn variant_parent_has_payload(&self, def: rynix_sema::DefId) -> bool {
        match self.analysis.defs.get(def.index() as usize) {
            Some(rynix_sema::DefKind::Variant { parent, .. }) => {
                self.analysis.enum_has_payload.contains(parent)
            }
            _ => false,
        }
    }

    fn span_text(&self, span: rynix_span::Span) -> &str {
        let lo = (span.lo().saturating_sub(self.base)) as usize;
        let hi = (span.hi().saturating_sub(self.base)) as usize;
        let hi = hi.min(self.src.len());
        let lo = lo.min(hi);
        &self.src[lo..hi]
    }
}
