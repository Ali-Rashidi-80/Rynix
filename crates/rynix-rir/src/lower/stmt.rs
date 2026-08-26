impl LowerCtx<'_, '_> {
    fn lower_guarded_loop(
        &mut self,
        span: rynix_span::Span,
        guard: LoopExitGuard,
        body: &[Stmt<'_>],
    ) {
        if let LoopExitGuard::Zero { counter: bit_sym } = guard {
            if let Some((accum_sym, bit2)) = try_parse_popcount_body(body) {
                if bit2 == bit_sym && self.sym_starts_at_zero(accum_sym) {
                    let v_init = self.load_sym(bit_sym);
                    let count = self.b.push_value(Inst::CtPop(v_init));
                    self.store_sym(accum_sym, count);
                    return;
                }
            }
        }
        let (extra_guards, body) = peel_rem_zero_guards(body);
        let guard_clears = collect_guard_clears(&extra_guards);
        let linear_carried =
            loop_carried_is_linear(body) && self.loops.iter().all(|f| f.linear_carried);
        if !linear_carried {
            self.materialize_all_mut_ssa(span);
        }
        let carried = self.collect_carried(body);

        let header = self.b.create_block();
        let loop_body = self.b.create_block();
        let exit_flush = self.b.create_block();
        let merge_exit_with_flush = guard_clears.is_empty();
        let exit = if merge_exit_with_flush {
            exit_flush
        } else {
            self.b.create_block()
        };
        let exit_clear_params: Vec<(Symbol, ValueId)> = guard_clears
            .iter()
            .map(|(sym, _)| (*sym, self.b.append_block_param(exit, IrTy::I64)))
            .collect();

        let flush_params: Vec<ValueId> = carried
            .iter()
            .map(|(_, _, ty)| self.b.append_block_param(exit_flush, *ty))
            .collect();

        let mut params = Vec::new();
        let mut init_args = Vec::new();
        for (sym, _, ty) in &carried {
            let param = self.b.append_block_param(header, *ty);
            params.push(param);
            init_args.push(self.load_sym(*sym));
        }

        self.b.jump(header, init_args);
        self.b.switch_to(header);
        self.b.seal_block(header);
        self.begin_loop_carried(&carried, &params, linear_carried);
        if !linear_carried {
            self.sync_carried_params_to_alloca();
        }

        let primary_cond = self.guard_continue_cond(guard);
        let exit_args = self.backedge_args();
        let first_check = if extra_guards.is_empty() {
            loop_body
        } else {
            self.b.create_block()
        };
        self.b.br(
            primary_cond,
            first_check,
            vec![],
            exit_flush,
            exit_args.clone(),
        );

        let mut chain_block = first_check;
        for (idx, extra) in extra_guards.iter().enumerate() {
            self.b.switch_to(chain_block);
            self.b.seal_block(chain_block);
            let cond = self.guard_continue_cond(*extra);
            let is_last = idx + 1 == extra_guards.len();
            let on_continue = if is_last {
                loop_body
            } else {
                self.b.create_block()
            };
            if let LoopExitGuard::RemZero {
                clear_sym,
                clear_val,
                ..
            } = extra
            {
                let clear_block = self.b.create_block();
                self.b.br(cond, on_continue, vec![], clear_block, vec![]);
                self.b.switch_to(clear_block);
                self.b.seal_block(clear_block);
                let merge = self.guard_clear_exit_args(
                    &guard_clears,
                    Some((*clear_sym, *clear_val)),
                );
                self.b.jump(exit, merge);
            } else {
                self.b.br(cond, on_continue, vec![], exit_flush, exit_args.clone());
            }
            chain_block = on_continue;
        }

        self.b.switch_to(loop_body);
        self.b.seal_block(loop_body);
        self.loops.push(LoopFrame {
            header,
            exit,
            linear_carried,
            guard_clears: guard_clears.clone(),
        });
        for s in body {
            self.stmt(s);
        }
        if !self.is_terminated() {
            let args = self.backedge_args();
            self.b.jump(header, args);
        }
        self.loops.pop();
        self.end_loop_carried();

        self.b.switch_to(exit_flush);
        self.b.seal_block(exit_flush);
        for ((sym, slot, _), fp) in carried.iter().zip(flush_params.iter()) {
            if let Some(slot) = slot {
                self.b.store(*slot, *fp);
            } else {
                self.locals.insert(*sym, Local::MutSsa(*fp));
            }
            self.sync_enclosing_carried(*sym, *fp);
        }
        let merge = self.guard_clear_exit_args(&guard_clears, None);
        if !merge_exit_with_flush {
            self.b.jump(exit, merge);
            self.b.switch_to(exit);
            self.b.seal_block(exit);
        }
        for (sym, param) in exit_clear_params {
            self.locals.insert(sym, Local::MutSsa(param));
        }
    }

    fn sync_enclosing_carried(&mut self, sym: Symbol, val: ValueId) {
        for frame in self.loop_carried.iter_mut() {
            if let Some(c) = frame.iter_mut().find(|c| c.sym == sym) {
                c.current = val;
                c.nonneg = self.mut_nonneg_syms.contains(&sym);
                c.strictly_positive = self.mut_positive_syms.contains(&sym);
            }
        }
    }

    fn load_slot(&mut self, slot: ValueId) -> ValueId {
        self.b.load(slot)
    }

    fn store_slot(&mut self, slot: ValueId, val: ValueId) {
        self.b.store(slot, val);
    }

    fn backedge_args(&mut self) -> Vec<ValueId> {
        if self.loop_uses_alloca_carried() {
            self.loop_carried
                .last()
                .map(|frame| {
                    frame
                        .iter()
                        .map(|c| self.b.load(c.slot.expect("alloca carried")))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.loop_carried
                .last()
                .map(|frame| frame.iter().map(|c| c.current).collect())
                .unwrap_or_default()
        }
    }

    fn materialize_all_mut_ssa(&mut self, span: rynix_span::Span) {
        let pending: Vec<(Symbol, ValueId)> = self
            .locals
            .iter()
            .filter_map(|(&sym, local)| {
                if let Local::MutSsa(v) = local {
                    Some((sym, *v))
                } else {
                    None
                }
            })
            .collect();
        for (sym, val) in pending {
            self.materialize_mut_ssa(sym, IrTy::I64, span, val);
        }
    }

    fn sync_carried_params_to_alloca(&mut self) {
        if let Some(frame) = self.loop_carried.last() {
            for c in frame.iter() {
                if let Some(slot) = c.slot {
                    self.b.store(slot, c.param);
                }
            }
        }
    }

    fn flush_carried_for_break(&mut self) {
        if self.loop_carried_linear.last().is_some_and(|&linear| linear) {
            if let Some(frame) = self.loop_carried.last() {
                for c in frame.iter() {
                    if let Some(slot) = c.slot {
                        self.b.store(slot, c.current);
                    } else {
                        self.locals.insert(c.sym, Local::MutSsa(c.current));
                    }
                }
            }
        } else {
            let args = self.backedge_args();
            if let Some(carried) = self.loop_carried.last() {
                for (c, &arg) in carried.iter().zip(args.iter()) {
                    if let Some(slot) = c.slot {
                        self.b.store(slot, arg);
                    }
                }
            }
        }
    }

    fn begin_loop_carried(
        &mut self,
        carried: &[(Symbol, Option<ValueId>, IrTy)],
        params: &[ValueId],
        linear: bool,
    ) {
        let frame: Vec<LoopCarried> = carried
            .iter()
            .zip(params.iter())
            .map(|((sym, slot, _), &param)| LoopCarried {
                sym: *sym,
                slot: *slot,
                param,
                current: param,
                nonneg: self.mut_nonneg_syms.contains(sym),
                strictly_positive: self.mut_positive_syms.contains(sym),
                excl_bound: self.mut_excl_bound.get(sym).copied(),
            })
            .collect();
        self.loop_carried.push(frame);
        self.loop_carried_linear.push(linear);
    }

    fn end_loop_carried(&mut self) {
        self.loop_carried.pop();
        self.loop_carried_linear.pop();
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        if self.is_terminated() {
            return;
        }
        match stmt {
            Stmt::Error(_) => {}
            Stmt::Break(_) => {
                if let Some(frame) = self.loops.last().cloned() {
                    self.flush_carried_for_break();
                    let args = if frame.guard_clears.is_empty() {
                        self.backedge_args()
                    } else {
                        self.guard_clear_exit_args(&frame.guard_clears, None)
                    };
                    self.b.jump(frame.exit, args);
                } else {
                    let _ = self.b.push(Inst::Unreachable);
                }
            }
            Stmt::Continue(_) => {
                if let Some(frame) = self.loops.last().cloned() {
                    let args = self.backedge_args();
                    self.b.jump(frame.header, args);
                } else {
                    let _ = self.b.push(Inst::Unreachable);
                }
            }
            Stmt::Let(l) => {
                let init = self.expr(l.init);
                if l.mutable {
                    if let Expr::Path(p) = l.init
                        && let Some(seg) = p.segments.last()
                    {
                        if self.mut_nonneg_syms.contains(&seg.name) {
                            self.mut_nonneg_syms.insert(l.name.name);
                        }
                        if self.mut_positive_syms.contains(&seg.name) {
                            self.mut_positive_syms.insert(l.name.name);
                        }
                    }
                    if self.value_is_nonneg(init) {
                        self.mut_nonneg_syms.insert(l.name.name);
                    }
                    if self.value_is_strictly_positive(init) {
                        self.mut_positive_syms.insert(l.name.name);
                    }
                    if let Some(b) = self.value_excl_bound(init) {
                        self.mut_excl_bound.insert(l.name.name, b);
                    }
                    let ty = self
                        .analysis
                        .node_types
                        .get(&l.id)
                        .map(|&t| map_ty(self.analysis, t))
                        .unwrap_or(IrTy::I64);
                    let site = self.b.reserve_stack_binding(ty, l.span);
                    self.mut_binding_sites.insert(l.name.name, site);
                    self.locals.insert(l.name.name, Local::MutSsa(init));
                } else {
                    // Immutable → direct SSA binding (Braun-style).
                    self.locals.insert(l.name.name, Local::Ssa(init));
                }
            }
            Stmt::Assign(a) => {
                if let Expr::Path(p) = a.target
                    && let Some(seg) = p.segments.last()
                {
                    let sym = seg.name;
                    let rhs = self.expr(a.value);
                    let val = match self.locals.get(&sym).copied() {
                        Some(Local::Slot(slot)) => {
                            let v = match a.op {
                                AssignOp::Eq => rhs,
                                AssignOp::PlusEq => {
                                    let cur = self.load_slot(slot);
                                    self.b.push_value(Inst::IAdd(cur, rhs))
                                }
                                AssignOp::MinusEq => {
                                    let cur = self.load_slot(slot);
                                    self.b.push_value(Inst::ISub(cur, rhs))
                                }
                                AssignOp::StarEq => {
                                    let cur = self.load_slot(slot);
                                    self.lower_int_mul(cur, rhs)
                                }
                                AssignOp::SlashEq => {
                                    let cur = self.load_slot(slot);
                                    self.lower_int_div(cur, rhs)
                                }
                                AssignOp::PercentEq => {
                                    let cur = self.load_slot(slot);
                                    self.lower_int_rem(cur, rhs)
                                }
                            };
                            self.update_nonneg_sym(sym, a.op, rhs, v);
                            self.update_positive_sym(sym, a.op, v);
                            self.store_slot(slot, v);
                            return;
                        }
                        Some(Local::MutSsa(_)) => match a.op {
                            AssignOp::Eq => rhs,
                            AssignOp::PlusEq => {
                                let cur = self.load_sym(sym);
                                self.b.push_value(Inst::IAdd(cur, rhs))
                            }
                            AssignOp::MinusEq => {
                                let cur = self.load_sym(sym);
                                self.b.push_value(Inst::ISub(cur, rhs))
                            }
                            AssignOp::StarEq => {
                                let cur = self.load_sym(sym);
                                self.lower_int_mul(cur, rhs)
                            }
                            AssignOp::SlashEq => {
                                let cur = self.load_sym(sym);
                                self.lower_int_div(cur, rhs)
                            }
                            AssignOp::PercentEq => {
                                let cur = self.load_sym(sym);
                                self.lower_int_rem(cur, rhs)
                            }
                        },
                        _ => rhs,
                    };
                    if matches!(self.locals.get(&sym), Some(Local::MutSsa(_))) {
                        self.update_nonneg_sym(sym, a.op, rhs, val);
                        self.update_positive_sym(sym, a.op, val);
                        self.store_sym(sym, val);
                    }
                } else if let Expr::Field(f) = a.target {
                    let rhs = self.expr(a.value);
                    let slot = self.lower_field_slot(f);
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
                            self.lower_int_mul(cur, rhs)
                        }
                        AssignOp::SlashEq => {
                            let cur = self.b.load(slot);
                            self.lower_int_div(cur, rhs)
                        }
                        AssignOp::PercentEq => {
                            let cur = self.b.load(slot);
                            self.lower_int_rem(cur, rhs)
                        }
                    };
                    self.b.store(slot, val);
                } else if let Expr::Index(i) = a.target {
                    // Phase 17-B: `a[i] = …` on array/slice layout (len + elems).
                    let base = self.expr(i.base);
                    let index = self.expr(i.index);
                    let len = self.b.push_value(Inst::ArrayLen(base));
                    let _ = self.b.push(Inst::BoundsCheck { index, len });
                    let one = self.b.iconst(1);
                    let off = self.b.push_value(Inst::IAdd(index, one));
                    let slot = self.b.push_value(Inst::GepI64 {
                        base,
                        index: off,
                    });
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
                            self.lower_int_mul(cur, rhs)
                        }
                        AssignOp::SlashEq => {
                            let cur = self.b.load(slot);
                            self.lower_int_div(cur, rhs)
                        }
                        AssignOp::PercentEq => {
                            let cur = self.b.load(slot);
                            self.lower_int_rem(cur, rhs)
                        }
                    };
                    self.b.store(slot, val);
                } else {
                    let _ = self.expr(a.value);
                }
            }
            Stmt::Return(r) => {
                let v = r.value.map(|e| self.expr(e));
                if self.inlining {
                    let ret_v = v.unwrap_or_else(|| self.b.iconst(0));
                    if let Some(merge) = self.inline_merge {
                        // Pass return value as merge block arg (SSA-correct across preds).
                        self.b.jump(merge, vec![ret_v]);
                        self.inline_ret = Some(ret_v); // marker: at least one return seen
                    } else if !self.is_terminated() {
                        let _ = self.b.push(Inst::Unreachable);
                    }
                    return;
                }
                self.b.ret(v);
            }
            Stmt::Expr(e) => {
                let _ = self.expr(e.expr);
            }
            Stmt::If(i) => {
                if let Some((target, cond)) = try_parse_conditional_add(i) {
                    self.lower_conditional_add(target, cond);
                    return;
                }
                self.lower_if(i);
            }
            Stmt::Match(m) => {
                self.lower_match(m);
            }
            Stmt::Loop(l) => {
                if let Some((guard, rest)) = try_parse_loop_exit_guard(l.body) {
                    // Prime's nested square-gt trial is rejected by `rest_allows_guarded_loop`;
                    // fold it before that gate when `limit` is a compile-time constant.
                    if let LoopExitGuard::CountedGt { counter, bound } = guard
                        && let Some(limit) = self.sym_iconst(bound)
                    {
                        if self.lower_prime_count_folded(counter, limit, rest)
                            || self.lower_gcd_sum_folded(counter, limit, rest)
                        {
                            return;
                        }
                    }
                    if self.loop_guard_eligible(guard) && rest_allows_guarded_loop(rest) {
                        if let LoopExitGuard::CountedGeLit { counter, bound } = guard
                            && self.lower_unrolled_counted_ge_loop(counter, bound, rest)
                        {
                            return;
                        }
                        if let Some((counter, bound)) = self.counted_ge_lit_bound(guard)
                            && self.lower_sum_of_squares_closed(counter, bound, rest)
                        {
                            return;
                        }
                        if let Some((counter, bound)) = self.counted_ge_lit_bound(guard)
                            && self.lower_const_trip_suite5_kernels(counter, bound, rest)
                        {
                            return;
                        }
                        // Dynamic-n strength reduction (opaque Suite5 trip counts).
                        if let Some((counter, bound)) = self.counted_ge_bound_sym(guard) {
                            if self.lower_scan_count_closed_dyn(counter, bound, rest)
                                || self.lower_sum_of_squares_closed_dyn(counter, bound, rest)
                                || self.lower_linear_mix_closed_dyn(counter, bound, rest)
                                || self.lower_hash_poly_closed_dyn(counter, bound, rest)
                                || self.lower_nested_ij_mod_dyn(counter, bound, rest)
                                || self.lower_powmod_bin_dyn(counter, bound, rest)
                                || self.lower_fib_matrix_dyn(counter, bound, rest)
                            {
                                return;
                            }
                        }
                        self.lower_guarded_loop(l.span, guard, rest);
                        return;
                    }
                }
                let linear_carried = self.loop_is_linear(&l.body);
                if !linear_carried {
                    self.materialize_all_mut_ssa(l.span);
                }
                let carried = self.collect_carried(l.body);
                let header = self.b.create_block();
                let exit = self.b.create_block();
                let exit_carried: Vec<(Symbol, ValueId)> = if body_has_break(l.body) {
                    carried
                        .iter()
                        .map(|(sym, _, ty)| (*sym, self.b.append_block_param(exit, *ty)))
                        .collect()
                } else {
                    Vec::new()
                };

                let mut params = Vec::new();
                let mut init_args = Vec::new();
                for (sym, _, ty) in &carried {
                    let param = self.b.append_block_param(header, *ty);
                    params.push(param);
                    init_args.push(self.load_sym(*sym));
                }

                self.b.jump(header, init_args);
                self.b.switch_to(header);
                self.b.seal_block(header);
                self.begin_loop_carried(&carried, &params, linear_carried);
                if !linear_carried {
                    self.sync_carried_params_to_alloca();
                }

                self.loops.push(LoopFrame {
                    header,
                    exit,
                    linear_carried,
                    guard_clears: Vec::new(),
                });
                for s in l.body {
                    self.stmt(s);
                }
                if !self.is_terminated() {
                    let args = self.backedge_args();
                    self.b.jump(header, args);
                }
                self.loops.pop();
                self.end_loop_carried();
                self.b.switch_to(exit);
                self.b.seal_block(exit);
                for (sym, param) in exit_carried {
                    self.locals.insert(sym, Local::MutSsa(param));
                }
            }
            Stmt::Region(r) => {
                let _ = self.b.push(Inst::RegionCreate { region: 0 });
                for s in r.body {
                    self.stmt(s);
                    if self.is_terminated() {
                        break;
                    }
                }
                if !self.is_terminated() {
                    let _ = self.b.push(Inst::RegionReset { region: 0 });
                }
            }
            Stmt::For(f) => {
                let base = self.expr(f.iter);
                let len = self.b.push_value(Inst::ArrayLen(base));
                let i_slot = self.b.alloc(IrTy::I64, f.span);
                let zero = self.b.iconst(0);
                self.b.store(i_slot, zero);
                self.mut_slots.insert(i_slot);
                let i_sym = self.interner.intern(&format!("__for_i{}", i_slot.0));
                self.locals.insert(i_sym, Local::Slot(i_slot));
                let binder_slot = self.b.alloc(IrTy::I64, f.span);
                self.mut_slots.insert(binder_slot);
                self.locals
                    .insert(f.binder.name, Local::Slot(binder_slot));

                let linear_carried = self.loop_is_linear(&f.body);
                if !linear_carried {
                    self.materialize_all_mut_ssa(f.span);
                }
                let carried = self.collect_carried(f.body);
                let header = self.b.create_block();
                let loop_body = self.b.create_block();
                let exit_flush = self.b.create_block();
                let exit = self.b.create_block();

                let flush_params: Vec<ValueId> = carried
                    .iter()
                    .map(|(_, _, ty)| self.b.append_block_param(exit_flush, *ty))
                    .collect();

                let mut params = Vec::new();
                let mut init_args = Vec::new();
                for (sym, _, ty) in &carried {
                    let param = self.b.append_block_param(header, *ty);
                    params.push(param);
                    init_args.push(self.load_sym(*sym));
                }

                self.b.jump(header, init_args);
                self.b.switch_to(header);
                self.b.seal_block(header);
                self.begin_loop_carried(&carried, &params, linear_carried);
                if !linear_carried {
                    self.sync_carried_params_to_alloca();
                }

                let i = self.load_slot(i_slot);
                let exit_args = self.backedge_args();
                let cond = self.b.push_value(Inst::ICmp(CmpOp::Lt, i, len));
                self.b.br(cond, loop_body, vec![], exit_flush, exit_args);
                self.b.switch_to(loop_body);
                self.b.seal_block(loop_body);
                let _ = self.b.push(Inst::BoundsCheck { index: i, len });
                let elem = self.b.push_value(Inst::LoadIndex { base, index: i });
                self.store_slot(binder_slot, elem);
                self.loops.push(LoopFrame {
                    header,
                    exit,
                    linear_carried,
                    guard_clears: Vec::new(),
                });
                for s in f.body {
                    self.stmt(s);
                }
                if !self.is_terminated() {
                    let i2 = self.load_slot(i_slot);
                    let one = self.b.iconst(1);
                    let next = self.b.push_value(Inst::IAdd(i2, one));
                    self.store_slot(i_slot, next);
                    let args = self.backedge_args();
                    self.b.jump(header, args);
                }
                self.loops.pop();
                self.end_loop_carried();
                self.b.switch_to(exit_flush);
                self.b.seal_block(exit_flush);
                for ((sym, slot, _), fp) in carried.iter().zip(flush_params.iter()) {
                    if let Some(slot) = slot {
                        self.b.store(*slot, *fp);
                    } else {
                        self.locals.insert(*sym, Local::MutSsa(*fp));
                    }
                    self.sync_enclosing_carried(*sym, *fp);
                }
                self.b.jump(exit, vec![]);
                self.b.switch_to(exit);
                self.b.seal_block(exit);
            }
        }
    }

    fn lower_if(&mut self, i: &rynix_ast::IfStmt<'_>) {
        // MutSsa is invalid across CFG joins (then/else). Promote to alloca like
        // non-linear loops so stores/loads stay well-defined at `join`.
        if if_stmt_updates_mut(i) {
            self.materialize_all_mut_ssa(i.span);
        }
        // Flatten to nested br for the first arm; elif/else chain.
        let join = self.b.create_block();
        self.lower_if_arms(&i.arms, i.else_body, join);
        self.finish_cfg_join(join);
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
        let cond = self.lower_lazy_bool(arm.cond);
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

    fn lower_match(&mut self, m: &rynix_ast::MatchStmt<'_>) {
        if match_stmt_updates_mut(m) {
            self.materialize_all_mut_ssa(m.span);
        }
        let join = self.b.create_block();
        let scrut = self.expr(m.scrutinee);
        self.lower_match_arms(scrut, m.arms, m.else_body, join);
        self.finish_cfg_join(join);
    }

    /// Seal a CFG join. If every arm returned/diverged (no edge into `join`),
    /// mark it `unreachable` so inlining does not add a phantom fallthrough
    /// predecessor to `inline_merge` (Phase 22 — clang phi `%bN` undefined).
    fn finish_cfg_join(&mut self, join: crate::ir::BlockId) {
        self.b.switch_to(join);
        if !self.is_terminated() && !block_has_incoming(&self.b.func, join) {
            let _ = self.b.push(Inst::Unreachable);
        }
        self.b.seal_block(join);
    }

    fn lower_match_arms(
        &mut self,
        scrut: ValueId,
        arms: &[rynix_ast::MatchArm<'_>],
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
        match &arm.pattern {
            rynix_ast::MatchPat::Wildcard(_) => {
                let then_b = self.b.create_block();
                self.b.jump(then_b, vec![]);
                self.b.switch_to(then_b);
                self.b.seal_block(then_b);
                for s in arm.body {
                    self.stmt(s);
                }
                if !self.is_terminated() {
                    self.b.jump(join, vec![]);
                }
            }
            rynix_ast::MatchPat::Literal(pat) => {
                let then_b = self.b.create_block();
                let else_b = self.b.create_block();
                let pval = self.expr(pat);
                let floaty = self.b.func.value_ty(scrut) == IrTy::F64
                    || self.b.func.value_ty(pval) == IrTy::F64;
                let cond = self.cmp(CmpOp::Eq, scrut, pval, floaty);
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
                self.lower_match_arms(scrut, &arms[1..], else_body, join);
            }
        }
    }

}
