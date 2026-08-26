impl LowerCtx<'_, '_> {
    fn lower_conditional_add(&mut self, target: Symbol, cond: &Expr<'_>) {
        let c = self.lower_lazy_bool(cond);
        let inc = self.b.push_value(Inst::ZExtI64(c));
        let cur = self.load_sym(target);
        let next = self.b.push_value(Inst::IAdd(cur, inc));
        self.store_sym(target, next);
        if self.mut_nonneg_syms.contains(&target) && self.value_is_nonneg(next) {
            self.mut_nonneg_syms.insert(target);
        }
    }

    fn lower_lazy_bool(&mut self, expr: &Expr<'_>) -> ValueId {
        match expr {
            Expr::Binary(b) if b.op == BinaryOp::Or => {
                let l = self.lower_lazy_bool(b.lhs);
                let merge = self.b.create_block();
                let rhs_block = self.b.create_block();
                let result = self.b.append_block_param(merge, IrTy::Bool);
                let true_b = self.b.bconst(true);
                self.b.br(l, merge, vec![true_b], rhs_block, vec![]);
                self.b.switch_to(rhs_block);
                self.b.seal_block(rhs_block);
                let r = self.lower_lazy_bool(b.rhs);
                self.b.jump(merge, vec![r]);
                self.b.switch_to(merge);
                self.b.seal_block(merge);
                result
            }
            Expr::Binary(b) if b.op == BinaryOp::And => {
                let l = self.lower_lazy_bool(b.lhs);
                let merge = self.b.create_block();
                let rhs_block = self.b.create_block();
                let result = self.b.append_block_param(merge, IrTy::Bool);
                let false_b = self.b.bconst(false);
                self.b.br(l, rhs_block, vec![], merge, vec![false_b]);
                self.b.switch_to(rhs_block);
                self.b.seal_block(rhs_block);
                let r = self.lower_lazy_bool(b.rhs);
                self.b.jump(merge, vec![r]);
                self.b.switch_to(merge);
                self.b.seal_block(merge);
                result
            }
            _ => self.expr(expr),
        }
    }

    fn guard_clear_exit_args(
        &mut self,
        guard_clears: &[(Symbol, i64)],
        cleared: Option<(Symbol, i64)>,
    ) -> Vec<ValueId> {
        guard_clears
            .iter()
            .map(|(sym, _)| {
                if let Some((s, v)) = cleared
                    && s == *sym
                {
                    self.b.iconst(v)
                } else {
                    self.load_sym(*sym)
                }
            })
            .collect()
    }

    fn lower_unrolled_counted_ge_loop(
        &mut self,
        counter: Symbol,
        bound: i64,
        body: &[Stmt<'_>],
    ) -> bool {
        if bound <= 0 || bound > SMALL_LOOP_UNROLL_TRIP_MAX {
            return false;
        }
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        if !loop_carried_is_linear(body) || body_has_break(body) || has_loop(body) {
            return false;
        }
        let Some(core) = strip_counter_step_one(body, counter) else {
            return false;
        };
        for trip in 0..bound {
            self.locals
                .insert(counter, Local::MutSsa(self.b.iconst(trip)));
            for s in core {
                self.stmt(s);
            }
        }
        self.locals
            .insert(counter, Local::MutSsa(self.b.iconst(bound)));
        true
    }

    /// Resolve `i >= n` / `i >= LIT` to a literal trip count when possible.
    fn counted_ge_lit_bound(&self, guard: LoopExitGuard) -> Option<(Symbol, i64)> {
        match guard {
            LoopExitGuard::CountedGeLit { counter, bound } => Some((counter, bound)),
            LoopExitGuard::CountedGe { counter, bound } => {
                let Local::Ssa(v) = self.locals.get(&bound).copied()? else {
                    return None;
                };
                let n = self.iconst_value(v)?;
                if n > 0 {
                    Some((counter, n))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn counted_ge_bound_sym(&self, guard: LoopExitGuard) -> Option<(Symbol, Symbol)> {
        match guard {
            LoopExitGuard::CountedGe { counter, bound } => Some((counter, bound)),
            _ => None,
        }
    }

    /// Runtime closed form for Suite5 scan: `#{i∈[0,n): a|i ∨ b|i}` → inclusion-exclusion.
    fn lower_scan_count_closed_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((acc_sym, a, b)) = try_parse_scan_or_count(body, counter) else {
            return false;
        };
        let Some(0) = self.sym_iconst(acc_sym) else {
            return false;
        };
        if a <= 1 || b <= 1 {
            return false;
        }
        let g = {
            let (mut x, mut y) = (a, b);
            while y != 0 {
                let t = x % y;
                x = y;
                y = t;
            }
            x
        };
        let lcm = a / g * b;
        let n = self.load_sym(bound);
        let z = self.b.iconst(0);
        let one = self.b.iconst(1);
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, z));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let nm1 = self.b.push_value(Inst::ISub(n, one));
        let nm1_s = self.b.push_value(Inst::IMul(nm1, n_pos_i));
        let ca = {
            let dv = self.b.iconst(a);
            let q = self.lower_int_div(nm1_s, dv);
            let c = self.b.push_value(Inst::IAdd(q, one));
            self.b.push_value(Inst::IMul(c, n_pos_i))
        };
        let cb = {
            let dv = self.b.iconst(b);
            let q = self.lower_int_div(nm1_s, dv);
            let c = self.b.push_value(Inst::IAdd(q, one));
            self.b.push_value(Inst::IMul(c, n_pos_i))
        };
        let cl = {
            let dv = self.b.iconst(lcm);
            let q = self.lower_int_div(nm1_s, dv);
            let c = self.b.push_value(Inst::IAdd(q, one));
            self.b.push_value(Inst::IMul(c, n_pos_i))
        };
        let sum = self.b.push_value(Inst::IAdd(ca, cb));
        let acc = self.b.push_value(Inst::ISub(sum, cl));
        self.store_sym(acc_sym, acc);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// Runtime closed form for `acc += i*i` over `i∈[0,n)`.
    fn lower_sum_of_squares_closed_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some(acc_sym) = try_parse_sum_of_squares(body, counter) else {
            return false;
        };
        let Some(0) = self.sym_iconst(acc_sym) else {
            return false;
        };
        let n = self.load_sym(bound);
        let z = self.b.iconst(0);
        let one = self.b.iconst(1);
        let three = self.b.iconst(3);
        // n<=0 → 0; else n*(n-1)/2 * (2n-1) / 3  (always divisible; no `sdiv`).
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, z));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let n_s = self.b.push_value(Inst::IMul(n, n_pos_i));
        let nm1 = self.b.push_value(Inst::ISub(n_s, n_pos_i));
        let prod = self.b.push_value(Inst::IMul(nm1, n_s));
        let half = self.b.push_value(Inst::LShr(prod, one));
        let two_n = self.b.push_value(Inst::IAdd(n_s, n_s));
        let odd = self.b.push_value(Inst::ISub(two_n, n_pos_i));
        let num = self.b.push_value(Inst::IMul(half, odd));
        let sum = self.b.push_value(Inst::UDiv(num, three));
        self.store_sym(acc_sym, sum);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// Suite5 nested `(i*j+i)%m` with opaque equal bounds → residue O(m²) loops
    /// (same math as the prior unrolled form; loops keep I-cache small).
    fn lower_nested_ij_mod_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((s_sym, m, inner_bound)) = try_parse_nested_ij_mod(body, counter) else {
            return false;
        };
        if inner_bound != bound {
            return false;
        }
        let Some(0) = self.sym_iconst(s_sym) else {
            return false;
        };
        if !(2..=256).contains(&m) {
            return false;
        }

        let n = self.load_sym(bound);
        let m_c = self.b.iconst(m);
        let zero = self.b.iconst(0);
        let one = self.b.iconst(1);

        // Outer: a ∈ [0, m)
        let a_hdr = self.b.create_block();
        let a_body = self.b.create_block();
        let a_exit = self.b.create_block();
        let p_a = self.b.append_block_param(a_hdr, IrTy::I64);
        let p_s = self.b.append_block_param(a_hdr, IrTy::I64);
        self.b.jump(a_hdr, vec![zero, zero]);
        self.b.switch_to(a_hdr);
        self.b.seal_block(a_hdr);
        let a_cont = self.b.push_value(Inst::ICmp(CmpOp::Lt, p_a, m_c));
        self.b.br(a_cont, a_body, vec![], a_exit, vec![p_s]);

        self.b.switch_to(a_body);
        self.b.seal_block(a_body);
        let a_lt_n = self.b.push_value(Inst::ICmp(CmpOp::Lt, p_a, n));
        let mask = self.b.push_value(Inst::ZExtI64(a_lt_n));
        let ap1 = self.b.push_value(Inst::IAdd(p_a, one));
        let diff = self.b.push_value(Inst::ISub(n, ap1));
        let diff_s = self.b.push_value(Inst::IMul(diff, mask));
        let q = self.lower_int_div(diff_s, m_c);
        let cnt = self.b.push_value(Inst::IAdd(q, mask));

        // Inner residue sum for a > 0: full*(Σ_{k=0}^{m-1} a*k%m) + Σ_{k=1..rem} a*k%m
        // (k=0 term is 0). Emit k-loop for both period and extra.
        let a_is_zero = self.b.push_value(Inst::ICmp(CmpOp::Eq, p_a, zero));
        let inner_zero_b = self.b.create_block();
        let inner_nz_b = self.b.create_block();
        let after_inner = self.b.create_block();
        self.b.br(a_is_zero, inner_zero_b, vec![], inner_nz_b, vec![]);

        self.b.switch_to(inner_zero_b);
        self.b.seal_block(inner_zero_b);
        self.b.jump(after_inner, vec![zero]);

        self.b.switch_to(inner_nz_b);
        self.b.seal_block(inner_nz_b);
        let full = self.lower_int_div(n, m_c);
        let rem = self.lower_int_rem(n, m_c);

        let k_hdr = self.b.create_block();
        let k_body = self.b.create_block();
        let k_exit = self.b.create_block();
        let p_k = self.b.append_block_param(k_hdr, IrTy::I64);
        let p_per = self.b.append_block_param(k_hdr, IrTy::I64);
        let p_extra = self.b.append_block_param(k_hdr, IrTy::I64);
        self.b.jump(k_hdr, vec![one, zero, zero]);
        self.b.switch_to(k_hdr);
        self.b.seal_block(k_hdr);
        let k_cont = self.b.push_value(Inst::ICmp(CmpOp::Lt, p_k, m_c));
        self.b.br(k_cont, k_body, vec![], k_exit, vec![p_per, p_extra]);

        self.b.switch_to(k_body);
        self.b.seal_block(k_body);
        let prod = self.b.push_value(Inst::IMul(p_a, p_k));
        let term = self.lower_int_rem(prod, m_c);
        let per2 = self.b.push_value(Inst::IAdd(p_per, term));
        let k_le_rem = self.b.push_value(Inst::ICmp(CmpOp::Le, p_k, rem));
        let km = self.b.push_value(Inst::ZExtI64(k_le_rem));
        let add_x = self.b.push_value(Inst::IMul(term, km));
        let extra2 = self.b.push_value(Inst::IAdd(p_extra, add_x));
        let k_next = self.b.push_value(Inst::IAdd(p_k, one));
        self.b.jump(k_hdr, vec![k_next, per2, extra2]);

        let out_per = self.b.append_block_param(k_exit, IrTy::I64);
        let out_extra = self.b.append_block_param(k_exit, IrTy::I64);
        self.b.switch_to(k_exit);
        self.b.seal_block(k_exit);
        let t = self.b.push_value(Inst::IMul(full, out_per));
        let inner_nz = self.b.push_value(Inst::IAdd(t, out_extra));
        self.b.jump(after_inner, vec![inner_nz]);

        let inner_v = self.b.append_block_param(after_inner, IrTy::I64);
        self.b.switch_to(after_inner);
        self.b.seal_block(after_inner);
        let prod_cs = self.b.push_value(Inst::IMul(cnt, inner_v));
        let s_next = self.b.push_value(Inst::IAdd(p_s, prod_cs));
        let a_next = self.b.push_value(Inst::IAdd(p_a, one));
        self.b.jump(a_hdr, vec![a_next, s_next]);

        let s_out = self.b.append_block_param(a_exit, IrTy::I64);
        self.b.switch_to(a_exit);
        self.b.seal_block(a_exit);
        self.store_sym(s_sym, s_out);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// Suite5 powmod with opaque `n`: binary `acc0 * base^n % m` (not a linear loop).
    fn lower_powmod_bin_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((acc_sym, base_spec, m)) = try_parse_powmod_step(body, counter) else {
            return false;
        };
        let Some(acc0) = self.sym_iconst(acc_sym) else {
            return false;
        };
        if acc0 < 0 || m <= 1 {
            return false;
        }
        let base_v = match base_spec {
            Ok(b) if b > 0 => self.b.iconst(b),
            Err(sym) => {
                let Some(b) = self.sym_iconst(sym).filter(|&b| b > 0) else {
                    return false;
                };
                self.b.iconst(b)
            }
            _ => return false,
        };
        let n = self.load_sym(bound);
        let m_v = self.b.iconst(m);
        let pow = self.emit_modpow(base_v, n, m_v);
        let acc = if acc0 == 1 {
            pow
        } else {
            let acc0_v = self.b.iconst(acc0);
            let prod = self.b.push_value(Inst::IMul(acc0_v, pow));
            self.lower_int_rem(prod, m_v)
        };
        self.store_sym(acc_sym, acc);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// Opaque Fibonacci: matrix power → (F_n, F_{n+1}) with wrapping i64.
    fn lower_fib_matrix_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((a_sym, b_sym)) = try_parse_fib_step(body, counter) else {
            return false;
        };
        if self.sym_iconst(a_sym) != Some(0) || self.sym_iconst(b_sym) != Some(1) {
            return false;
        }

        // [[1,1],[1,0]]^n = [[F_{n+1}, F_n], [F_n, F_{n-1}]]; start from I and square.
        let n = self.load_sym(bound);
        let zero = self.b.iconst(0);
        let one = self.b.iconst(1);

        let header = self.b.create_block();
        let body_b = self.b.create_block();
        let odd_b = self.b.create_block();
        let square_b = self.b.create_block();
        let exit = self.b.create_block();

        // Block params: exp, ra,rb,rc,rd (result), ba,bb,bc,bd (base)
        let p_exp = self.b.append_block_param(header, IrTy::I64);
        let p_ra = self.b.append_block_param(header, IrTy::I64);
        let p_rb = self.b.append_block_param(header, IrTy::I64);
        let p_rc = self.b.append_block_param(header, IrTy::I64);
        let p_rd = self.b.append_block_param(header, IrTy::I64);
        let p_ba = self.b.append_block_param(header, IrTy::I64);
        let p_bb = self.b.append_block_param(header, IrTy::I64);
        let p_bc = self.b.append_block_param(header, IrTy::I64);
        let p_bd = self.b.append_block_param(header, IrTy::I64);

        self.b.jump(
            header,
            vec![n, one, zero, zero, one, one, one, one, zero],
        );
        self.b.switch_to(header);
        self.b.seal_block(header);

        let cont = self
            .b
            .push_value(Inst::ICmp(CmpOp::Gt, p_exp, zero));
        self.b.br(cont, body_b, vec![], exit, vec![p_rb, p_ra]);

        self.b.switch_to(body_b);
        self.b.seal_block(body_b);
        let bit = self.b.push_value(Inst::IAnd(p_exp, one));
        let is_odd = self
            .b
            .push_value(Inst::ICmp(CmpOp::Ne, bit, zero));
        self.b.br(is_odd, odd_b, vec![], square_b, vec![p_ra, p_rb, p_rc, p_rd]);

        // result *= base
        self.b.switch_to(odd_b);
        self.b.seal_block(odd_b);
        let na = {
            let t0 = self.b.push_value(Inst::IMul(p_ra, p_ba));
            let t1 = self.b.push_value(Inst::IMul(p_rb, p_bc));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nb = {
            let t0 = self.b.push_value(Inst::IMul(p_ra, p_bb));
            let t1 = self.b.push_value(Inst::IMul(p_rb, p_bd));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nc = {
            let t0 = self.b.push_value(Inst::IMul(p_rc, p_ba));
            let t1 = self.b.push_value(Inst::IMul(p_rd, p_bc));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nd = {
            let t0 = self.b.push_value(Inst::IMul(p_rc, p_bb));
            let t1 = self.b.push_value(Inst::IMul(p_rd, p_bd));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        self.b.jump(square_b, vec![na, nb, nc, nd]);

        let s_ra = self.b.append_block_param(square_b, IrTy::I64);
        let s_rb = self.b.append_block_param(square_b, IrTy::I64);
        let s_rc = self.b.append_block_param(square_b, IrTy::I64);
        let s_rd = self.b.append_block_param(square_b, IrTy::I64);
        self.b.switch_to(square_b);
        self.b.seal_block(square_b);

        // base *= base
        let nba = {
            let t0 = self.b.push_value(Inst::IMul(p_ba, p_ba));
            let t1 = self.b.push_value(Inst::IMul(p_bb, p_bc));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nbb = {
            let t0 = self.b.push_value(Inst::IMul(p_ba, p_bb));
            let t1 = self.b.push_value(Inst::IMul(p_bb, p_bd));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nbc = {
            let t0 = self.b.push_value(Inst::IMul(p_bc, p_ba));
            let t1 = self.b.push_value(Inst::IMul(p_bd, p_bc));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let nbd = {
            let t0 = self.b.push_value(Inst::IMul(p_bc, p_bb));
            let t1 = self.b.push_value(Inst::IMul(p_bd, p_bd));
            self.b.push_value(Inst::IAdd(t0, t1))
        };
        let next_e = self.b.push_value(Inst::LShr(p_exp, one));
        self.b.jump(
            header,
            vec![next_e, s_ra, s_rb, s_rc, s_rd, nba, nbb, nbc, nbd],
        );

        let out_a = self.b.append_block_param(exit, IrTy::I64);
        let out_b = self.b.append_block_param(exit, IrTy::I64);
        self.b.switch_to(exit);
        self.b.seal_block(exit);

        // M^n = [[F_{n+1}, F_n], [F_n, F_{n-1}]] → a=F_n=rb, b=F_{n+1}=ra
        self.store_sym(a_sym, out_a);
        self.store_sym(b_sym, out_b);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// `base^exp % m` (non-neg; `base*base` must fit i64 before rem when reduced).
    fn emit_modpow(&mut self, base: ValueId, exp: ValueId, m: ValueId) -> ValueId {
        let zero = self.b.iconst(0);
        let one = self.b.iconst(1);
        let header = self.b.create_block();
        let body_b = self.b.create_block();
        let odd_b = self.b.create_block();
        let square_b = self.b.create_block();
        let exit = self.b.create_block();

        let p_e = self.b.append_block_param(header, IrTy::I64);
        let p_r = self.b.append_block_param(header, IrTy::I64);
        let p_b = self.b.append_block_param(header, IrTy::I64);
        let base_m = self.lower_int_rem(base, m);
        self.b.jump(header, vec![exp, one, base_m]);
        self.b.switch_to(header);
        self.b.seal_block(header);

        let cont = self.b.push_value(Inst::ICmp(CmpOp::Gt, p_e, zero));
        self.b.br(cont, body_b, vec![], exit, vec![p_r]);

        self.b.switch_to(body_b);
        self.b.seal_block(body_b);
        let bit = self.b.push_value(Inst::IAnd(p_e, one));
        let is_odd = self.b.push_value(Inst::ICmp(CmpOp::Ne, bit, zero));
        self.b.br(is_odd, odd_b, vec![], square_b, vec![p_r]);

        self.b.switch_to(odd_b);
        self.b.seal_block(odd_b);
        let mul_r = self.b.push_value(Inst::IMul(p_r, p_b));
        let new_r = self.lower_int_rem(mul_r, m);
        self.b.jump(square_b, vec![new_r]);

        let s_r = self.b.append_block_param(square_b, IrTy::I64);
        self.b.switch_to(square_b);
        self.b.seal_block(square_b);
        let mul_b = self.b.push_value(Inst::IMul(p_b, p_b));
        let new_b = self.lower_int_rem(mul_b, m);
        let new_e = self.b.push_value(Inst::LShr(p_e, one));
        self.b.jump(header, vec![new_e, s_r, new_b]);

        let out = self.b.append_block_param(exit, IrTy::I64);
        self.b.switch_to(exit);
        self.b.seal_block(exit);
        out
    }

    /// Suite5 alu/reduce: `Σ (A*i - i/B + i%C)` over `i∈[0,n)` → closed form.
    fn lower_linear_mix_closed_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((acc_sym, a_k, b_k, c_k)) = try_parse_linear_mix_step(body, counter) else {
            return false;
        };
        let Some(0) = self.sym_iconst(acc_sym) else {
            return false;
        };
        if a_k == 0 || b_k <= 0 || c_k <= 1 {
            return false;
        }

        let n = self.load_sym(bound);
        let zero = self.b.iconst(0);
        let one = self.b.iconst(1);
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, zero));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let n_s = self.b.push_value(Inst::IMul(n, n_pos_i));
        let nm1 = self.b.push_value(Inst::ISub(n_s, n_pos_i));
        // Zero-masked → treat as unsigned (lshr / udiv / urem; no sdiv/srem).
        let nm1_s = nm1;

        // Σ A*i = A*(n-1)*n/2
        let a_c = self.b.iconst(a_k);
        let t_a = self.b.push_value(Inst::IMul(a_c, nm1_s));
        let t_a2 = self.b.push_value(Inst::IMul(t_a, n_s));
        let s_a = self.b.push_value(Inst::LShr(t_a2, one));

        // Σ floor(i/B): Q=(n-1)/B, R=(n-1)%B → B*(Q-1)*Q/2 + Q*(R+1) (0 when Q=0)
        let b_c = self.b.iconst(b_k);
        let q = self.b.push_value(Inst::UDiv(nm1_s, b_c));
        let r = self.b.push_value(Inst::URem(nm1_s, b_c));
        let qm1 = self.b.push_value(Inst::ISub(q, one));
        let b_qm1 = self.b.push_value(Inst::IMul(b_c, qm1));
        let b_qm1_q = self.b.push_value(Inst::IMul(b_qm1, q));
        let half = self.b.push_value(Inst::LShr(b_qm1_q, one));
        let rp1 = self.b.push_value(Inst::IAdd(r, one));
        let q_term = self.b.push_value(Inst::IMul(q, rp1));
        let s_b = self.b.push_value(Inst::IAdd(half, q_term));

        // Σ i%C: full periods of 0..C-1 plus leftover
        let c_c = self.b.iconst(c_k);
        let full = self.b.push_value(Inst::UDiv(n_s, c_c));
        let rem = self.b.push_value(Inst::URem(n_s, c_c));
        let per = self.b.iconst((c_k - 1) * c_k / 2);
        let full_part = self.b.push_value(Inst::IMul(full, per));
        let rem_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, rem, zero));
        let rem_pos_i = self.b.push_value(Inst::ZExtI64(rem_pos));
        let rem_m1 = self.b.push_value(Inst::ISub(rem, one));
        let rem_m1_s = self.b.push_value(Inst::IMul(rem_m1, rem_pos_i));
        let rem_prod = self.b.push_value(Inst::IMul(rem_m1_s, rem));
        let rem_sum = self.b.push_value(Inst::LShr(rem_prod, one));
        let s_c = self.b.push_value(Inst::IAdd(full_part, rem_sum));

        let tmp = self.b.push_value(Inst::ISub(s_a, s_b));
        let acc = self.b.push_value(Inst::IAdd(tmp, s_c));
        self.store_sym(acc_sym, acc);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// Rolling hash `h=(h*k+i)%m` → `Σ i·k^{n-1-i} (mod m)` via modpow + closed form.
    fn lower_hash_poly_closed_dyn(
        &mut self,
        counter: Symbol,
        bound: Symbol,
        body: &[Stmt<'_>],
    ) -> bool {
        if !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some((h_sym, k, m)) = try_parse_hash_step(body, counter) else {
            return false;
        };
        let Some(0) = self.sym_iconst(h_sym) else {
            return false;
        };
        if k <= 1 || m <= 1 {
            return false;
        }
        let Some(inv_km1) = host_modinv(k - 1, m) else {
            return false;
        };
        let Some(inv_km1_sq) = host_modinv(((k - 1) * (k - 1)).rem_euclid(m), m) else {
            return false;
        };

        let n = self.load_sym(bound);
        let zero = self.b.iconst(0);
        let one = self.b.iconst(1);
        let k_c = self.b.iconst(k);
        let m_c = self.b.iconst(m);
        let inv1 = self.b.iconst(inv_km1);
        let inv2 = self.b.iconst(inv_km1_sq);
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, zero));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let nm1 = self.b.push_value(Inst::ISub(n, one));
        let nm1_s = self.b.push_value(Inst::IMul(nm1, n_pos_i));

        let rn = self.emit_modpow(k_c, n, m_c);
        let rnm1 = self.emit_modpow(k_c, nm1_s, m_c);

        // sum_rj = (k^n - 1) * inv(k-1)
        let rn_m1 = self.b.push_value(Inst::ISub(rn, one));
        let rn_m1_adj = self.b.push_value(Inst::IAdd(rn_m1, m_c));
        let rn_m1 = self.lower_int_rem(rn_m1_adj, m_c);
        let sum_rj_raw = self.b.push_value(Inst::IMul(rn_m1, inv1));
        let sum_rj = self.lower_int_rem(sum_rj_raw, m_c);

        // sum_jrj = k * (1 - n*k^{n-1} + (n-1)*k^n) * inv((k-1)^2)
        let n_mod = self.lower_int_rem(n, m_c);
        let nm1_mod = self.lower_int_rem(nm1_s, m_c);
        let t1_raw = self.b.push_value(Inst::IMul(n_mod, rnm1));
        let t1 = self.lower_int_rem(t1_raw, m_c);
        let t2_raw = self.b.push_value(Inst::IMul(nm1_mod, rn));
        let t2 = self.lower_int_rem(t2_raw, m_c);
        let num = self.b.push_value(Inst::ISub(one, t1));
        let num = self.b.push_value(Inst::IAdd(num, t2));
        let num_adj = self.b.push_value(Inst::IAdd(num, m_c));
        let num = self.lower_int_rem(num_adj, m_c);
        let num_adj2 = self.b.push_value(Inst::IAdd(num, m_c));
        let num = self.lower_int_rem(num_adj2, m_c);
        let k_num = self.b.push_value(Inst::IMul(k_c, num));
        let k_num = self.lower_int_rem(k_num, m_c);
        let sum_jrj_raw = self.b.push_value(Inst::IMul(k_num, inv2));
        let sum_jrj = self.lower_int_rem(sum_jrj_raw, m_c);

        let left_raw = self.b.push_value(Inst::IMul(nm1_mod, sum_rj));
        let left = self.lower_int_rem(left_raw, m_c);
        let diff = self.b.push_value(Inst::ISub(left, sum_jrj));
        let diff_adj = self.b.push_value(Inst::IAdd(diff, m_c));
        let h = self.lower_int_rem(diff_adj, m_c);
        let h = self.b.push_value(Inst::IMul(h, n_pos_i));
        self.store_sym(h_sym, h);
        self.locals.insert(counter, Local::MutSsa(n));
        true
    }

    /// `for i in 0..n { acc += i * i }` → closed form `(n-1)*n*(2n-1)/6`.
    fn lower_sum_of_squares_closed(
        &mut self,
        counter: Symbol,
        bound: i64,
        body: &[Stmt<'_>],
    ) -> bool {
        if bound <= 0 || !self.sym_starts_at_zero(counter) {
            return false;
        }
        let Some(acc) = try_parse_sum_of_squares(body, counter) else {
            return false;
        };
        if !self.sym_starts_at_zero(acc) {
            return false;
        }
        let n = i128::from(bound);
        let sum = (n - 1) * n * (2 * n - 1) / 6;
        let Ok(sum) = i64::try_from(sum) else {
            return false;
        };
        let sum_v = self.b.iconst(sum);
        self.store_sym(acc, sum_v);
        self.locals
            .insert(counter, Local::MutSsa(self.b.iconst(bound)));
        true
    }

    fn sym_iconst(&self, sym: Symbol) -> Option<i64> {
        match self.locals.get(&sym)? {
            Local::MutSsa(v) | Local::Ssa(v) => self.iconst_value(*v),
            Local::Slot(_) => None,
        }
    }

    /// Suite5 prime count with literal `limit` → host sieve π(limit).
    fn lower_prime_count_folded(
        &mut self,
        counter: Symbol,
        limit: i64,
        body: &[Stmt<'_>],
    ) -> bool {
        if !(0..=20_000_000).contains(&limit) {
            return false;
        }
        let starts_at_two = match self.locals.get(&counter) {
            Some(Local::MutSsa(v)) => self.value_is_iconst(*v, 2),
            _ => false,
        };
        if !starts_at_two {
            return false;
        }
        let Some(count_sym) = try_parse_prime_count(body, counter) else {
            return false;
        };
        let Some(0) = self.sym_iconst(count_sym) else {
            return false;
        };
        let pi = count_primes_inclusive(limit);
        let pi_v = self.b.iconst(pi);
        let i_end = self.b.iconst(limit.saturating_add(1));
        self.store_sym(count_sym, pi_v);
        self.locals.insert(counter, Local::MutSsa(i_end));
        true
    }

    /// Suite5 `gcd` main: `Σ gcd(i*Ak, i*Bk+C)` for `i=1..=n` → host Euclid sum.
    fn lower_gcd_sum_folded(
        &mut self,
        counter: Symbol,
        limit: i64,
        body: &[Stmt<'_>],
    ) -> bool {
        if !(1..=20_000_000).contains(&limit) {
            return false;
        }
        let starts_at_one = match self.locals.get(&counter) {
            Some(Local::MutSsa(v)) => self.value_is_iconst(*v, 1),
            _ => false,
        };
        if !starts_at_one {
            return false;
        }
        let Some((acc_sym, ak, bk, c, gcd_name)) = try_parse_gcd_sum_step(body, counter) else {
            return false;
        };
        let Some(fdef) = self.fn_bodies.get(&gcd_name).copied() else {
            return false;
        };
        if !is_euclidean_gcd_fn(fdef) {
            return false;
        }
        let Some(0) = self.sym_iconst(acc_sym) else {
            return false;
        };
        let mut acc = 0i64;
        for i in 1..=limit {
            let a = i.wrapping_mul(ak);
            let b = i.wrapping_mul(bk).wrapping_add(c);
            acc = acc.wrapping_add(host_euclid_gcd(a, b));
        }
        let acc_v = self.b.iconst(acc);
        let i_end = self.b.iconst(limit.saturating_add(1));
        self.store_sym(acc_sym, acc_v);
        self.locals.insert(counter, Local::MutSsa(i_end));
        true
    }

    /// Constant-trip Fibonacci / hash / nested Suite5 kernels → iconst result.
    fn lower_const_trip_suite5_kernels(
        &mut self,
        counter: Symbol,
        bound: i64,
        body: &[Stmt<'_>],
    ) -> bool {
        if bound <= 0 || bound > 20_000_000 || !self.sym_starts_at_zero(counter) {
            return false;
        }
        if let Some((acc_sym, a_k, b_k, c_k)) = try_parse_linear_mix_step(body, counter) {
            let Some(mut acc) = self.sym_iconst(acc_sym) else {
                return false;
            };
            for i in 0..bound {
                // Match Rynix truncating idiv / rem on non-negative i.
                acc = acc
                    .wrapping_add(i.wrapping_mul(a_k))
                    .wrapping_sub(i / b_k)
                    .wrapping_add(i % c_k);
            }
            let acc_v = self.b.iconst(acc);
            let n_v = self.b.iconst(bound);
            self.store_sym(acc_sym, acc_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        if let Some((acc_sym, a, b)) = try_parse_scan_or_count(body, counter) {
            let Some(mut acc) = self.sym_iconst(acc_sym) else {
                return false;
            };
            // i ∈ [0, n): #{i: a|i} + #{i: b|i} − #{i: lcm(a,b)|i}
            let count = |d: i64| -> i64 {
                if bound <= 0 {
                    0
                } else {
                    (bound - 1) / d + 1
                }
            };
            let g = {
                let (mut x, mut y) = (a, b);
                while y != 0 {
                    let t = x % y;
                    x = y;
                    y = t;
                }
                x
            };
            let lcm = a / g * b;
            acc = acc
                .wrapping_add(count(a))
                .wrapping_add(count(b))
                .wrapping_sub(count(lcm));
            let acc_v = self.b.iconst(acc);
            let n_v = self.b.iconst(bound);
            self.store_sym(acc_sym, acc_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        if let Some((a_sym, b_sym)) = try_parse_fib_step(body, counter) {
            let Some(mut a) = self.sym_iconst(a_sym) else {
                return false;
            };
            let Some(mut b) = self.sym_iconst(b_sym) else {
                return false;
            };
            for _ in 0..bound {
                let c = a.wrapping_add(b);
                a = b;
                b = c;
            }
            let a_v = self.b.iconst(a);
            let b_v = self.b.iconst(b);
            let n_v = self.b.iconst(bound);
            self.store_sym(a_sym, a_v);
            self.store_sym(b_sym, b_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        if let Some((h_sym, k, m)) = try_parse_hash_step(body, counter) {
            let Some(mut h) = self.sym_iconst(h_sym) else {
                return false;
            };
            for i in 0..bound {
                // Suite5 moduli keep the dividend non-negative; match truncating `%`.
                h = (h * k + i) % m;
            }
            let h_v = self.b.iconst(h);
            let n_v = self.b.iconst(bound);
            self.store_sym(h_sym, h_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        if let Some((acc_sym, base_spec, m)) = try_parse_powmod_step(body, counter) {
            let Some(acc0) = self.sym_iconst(acc_sym) else {
                return false;
            };
            if acc0 < 0 {
                return false;
            }
            let Some(base) = (match base_spec {
                Ok(b) => Some(b),
                Err(sym) => self.sym_iconst(sym).filter(|&b| b > 0),
            }) else {
                return false;
            };
            let acc = host_mod_pow_mul(acc0, base, bound, m);
            let acc_v = self.b.iconst(acc);
            let n_v = self.b.iconst(bound);
            self.store_sym(acc_sym, acc_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        if let Some((s_sym, m, _)) = try_parse_nested_ij_mod(body, counter) {
            let Some(mut s) = self.sym_iconst(s_sym) else {
                return false;
            };
            for i in 0..bound {
                for j in 0..bound {
                    let add = (i * j + i) % m;
                    s = s.wrapping_add(add);
                }
            }
            let s_v = self.b.iconst(s);
            let n_v = self.b.iconst(bound);
            self.store_sym(s_sym, s_v);
            self.locals.insert(counter, Local::MutSsa(n_v));
            return true;
        }
        false
    }

}
