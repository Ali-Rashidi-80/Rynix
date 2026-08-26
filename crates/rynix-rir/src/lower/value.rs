impl LowerCtx<'_, '_> {
    fn is_terminated(&self) -> bool {
        let block = self.b.func.block(self.b.current());
        block
            .insts
            .last()
            .is_some_and(|id| self.b.func.inst(*id).is_terminator())
    }

    fn slot_payload_ty(&self, slot: ValueId) -> IrTy {
        let Some(def) = self.b.func.value(slot).def else {
            return IrTy::I64;
        };
        match self.b.func.inst(def) {
            Inst::Alloc { ty, .. } => *ty,
            _ => IrTy::I64,
        }
    }

    fn collect_carried(&self, body: &[Stmt<'_>]) -> Vec<(Symbol, Option<ValueId>, IrTy)> {
        let updated = mut_syms_updated_in(body);
        let mut v: Vec<(Symbol, Option<ValueId>, IrTy)> = self
            .locals
            .iter()
            .filter_map(|(&sym, local)| {
                if !updated.contains(&sym) {
                    return None;
                }
                match local {
                    Local::Slot(slot) if self.mut_slots.contains(slot) => Some((
                        sym,
                        Some(*slot),
                        self.slot_payload_ty(*slot),
                    )),
                    Local::MutSsa(_) => Some((sym, None, IrTy::I64)),
                    _ => None,
                }
            })
            .collect();
        v.sort_by_key(|(sym, _, _)| sym.index());
        v
    }

    fn carried_frame_entry(&self, sym: Symbol) -> Option<&LoopCarried> {
        self.loop_carried
            .iter()
            .rev()
            .find_map(|frame| frame.iter().find(|c| c.sym == sym))
    }

    fn carried_current_sym(&self, sym: Symbol) -> Option<ValueId> {
        self.carried_frame_entry(sym).map(|c| c.current)
    }

    fn set_carried_current_sym(&mut self, sym: Symbol, val: ValueId) {
        if let Some(frame) = self.loop_carried.last_mut()
            && let Some(c) = frame.iter_mut().find(|c| c.sym == sym)
        {
            c.current = val;
            c.nonneg = self.mut_nonneg_syms.contains(&sym);
            c.strictly_positive = self.mut_positive_syms.contains(&sym);
            c.excl_bound = self.mut_excl_bound.get(&sym).copied();
        }
    }

    /// GEP to an i64 field slot of a struct value (pointer to consecutive i64s).
    fn lower_field_slot(&mut self, f: &FieldExpr<'_>) -> ValueId {
        let base = self.expr(f.base);
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
        self.b.push_value(Inst::GepI64 {
            base,
            index: idx,
        })
    }

    fn load_sym(&mut self, sym: Symbol) -> ValueId {
        if let Some(c) = self.carried_frame_entry(sym).copied() {
            if self.loop_uses_alloca_carried() {
                if let Some(slot) = c.slot {
                    return self.b.load(slot);
                }
            }
            return c.current;
        }
        match self.locals.get(&sym).copied() {
            Some(Local::Slot(slot)) => self.b.load(slot),
            Some(Local::MutSsa(v)) | Some(Local::Ssa(v)) => v,
            None => self.b.iconst(0),
        }
    }

    fn store_sym(&mut self, sym: Symbol, val: ValueId) {
        if self.carried_current_sym(sym).is_some() {
            self.set_carried_current_sym(sym, val);
            if self.loop_uses_alloca_carried()
                && let Some(slot) = self.carried_frame_entry(sym).and_then(|c| c.slot)
            {
                self.b.store(slot, val);
            }
        } else {
            match self.locals.get(&sym).copied() {
                Some(Local::Slot(slot)) => self.b.store(slot, val),
                Some(Local::MutSsa(_)) => {
                    self.locals.insert(sym, Local::MutSsa(val));
                }
                _ => {}
            }
        }
    }

    fn materialize_mut_ssa(&mut self, sym: Symbol, ty: IrTy, span: rynix_span::Span, val: ValueId) -> ValueId {
        let slot = if let Some(site) = self.mut_binding_sites.get(&sym).copied() {
            self.b.alloc_at_site(site, ty, span)
        } else {
            self.b.alloc(ty, span)
        };
        self.b.store(slot, val);
        self.mut_slots.insert(slot);
        self.locals.insert(sym, Local::Slot(slot));
        slot
    }

    fn value_is_iconst(&self, v: ValueId, n: i64) -> bool {
        let Some(def) = self.b.func.value(v).def else {
            return false;
        };
        matches!(self.b.func.inst(def), Inst::IConst(x) if *x == n)
    }

    fn value_is_nonneg_iconst(&self, v: ValueId) -> bool {
        let Some(def) = self.b.func.value(v).def else {
            return false;
        };
        matches!(self.b.func.inst(def), Inst::IConst(n) if *n >= 0)
    }

    fn iconst_pow2_shift(&self, v: ValueId) -> Option<u32> {
        let Some(def) = self.b.func.value(v).def else {
            return None;
        };
        let Inst::IConst(n) = self.b.func.inst(def) else {
            return None;
        };
        if *n > 0 && (*n & (*n - 1)) == 0 {
            Some(n.trailing_zeros())
        } else {
            None
        }
    }

    /// `n = 2^shift + 1` → `(shift, true)`.
    /// `2^shift - 1` (e.g. 31) stays as `imul` so `(h*31+i)%m` keeps LLVM rem magic.
    fn iconst_shift_mul_form(&self, v: ValueId) -> Option<(u32, bool)> {
        let Some(def) = self.b.func.value(v).def else {
            return None;
        };
        let Inst::IConst(n) = self.b.func.inst(def) else {
            return None;
        };
        if *n <= 0 {
            return None;
        }
        let plus_one = *n - 1;
        if plus_one > 0 && (plus_one & (plus_one - 1)) == 0 {
            return Some((plus_one.trailing_zeros(), true));
        }
        None
    }

    fn value_is_nonneg_carried(&self, v: ValueId) -> bool {
        self.loop_carried.iter().rev().any(|frame| {
            frame.iter().any(|c| {
                c.nonneg && (c.current == v || c.param == v)
            })
        })
    }

    fn value_is_strictly_positive(&self, v: ValueId) -> bool {
        if self.positive_iconst(v).is_some() {
            return true;
        }
        if self.loop_carried.iter().any(|frame| {
            frame.iter().any(|c| {
                c.strictly_positive && (c.current == v || c.param == v)
            })
        }) {
            return true;
        }
        let Some(def) = self.b.func.value(v).def else {
            return false;
        };
        match self.b.func.inst(def) {
            Inst::IConst(n) => *n > 0,
            Inst::IAdd(a, b) => {
                (self.value_is_strictly_positive(*a) && self.value_is_nonneg_iconst(*b))
                    || (self.value_is_strictly_positive(*b) && self.value_is_nonneg_iconst(*a))
                    || (self.value_is_strictly_positive(*a) && self.value_is_strictly_positive(*b))
            }
            Inst::IMul(a, b) => {
                (self.positive_iconst(*a).is_some() && self.value_is_strictly_positive(*b))
                    || (self.positive_iconst(*b).is_some() && self.value_is_strictly_positive(*a))
            }
            Inst::ISub(a, b) => {
                self.value_is_strictly_positive(*a)
                    && (self.value_is_nonneg_iconst(*b) || self.value_is_strictly_positive(*b))
            }
            _ => false,
        }
    }

    fn sym_starts_at_zero(&self, sym: Symbol) -> bool {
        match self.locals.get(&sym) {
            Some(Local::MutSsa(v)) | Some(Local::Ssa(v)) => self.value_is_iconst(*v, 0),
            Some(Local::Slot(slot)) => {
                // Stack slot — assume 0 only when never written (conservative false).
                let _ = slot;
                false
            }
            None => false,
        }
    }

    fn positive_iconst(&self, v: ValueId) -> Option<i64> {
        let def = self.b.func.value(v).def?;
        match self.b.func.inst(def) {
            Inst::IConst(n) if *n > 0 => Some(*n),
            _ => None,
        }
    }

    fn value_is_nonneg_rem_result(&self, v: ValueId) -> bool {
        let Some(def) = self.b.func.value(v).def else {
            return false;
        };
        match self.b.func.inst(def) {
            Inst::URem(a, b) | Inst::IRem(a, b) => {
                self.value_is_nonneg(*a)
                    && (self.positive_iconst(*b).is_some() || self.value_is_strictly_positive(*b))
            }
            Inst::IAnd(l, _) => self.value_is_nonneg(*l),
            _ => false,
        }
    }

    fn value_is_nonneg(&self, v: ValueId) -> bool {
        if self.value_is_nonneg_iconst(v) || self.value_is_nonneg_carried(v) {
            return true;
        }
        let Some(def) = self.b.func.value(v).def else {
            return false;
        };
        match self.b.func.inst(def) {
            Inst::IConst(n) => *n >= 0,
            Inst::URem(a, b) | Inst::IRem(a, b) => {
                self.value_is_nonneg(*a)
                    && (self.positive_iconst(*b).is_some() || self.value_is_strictly_positive(*b))
            }
            Inst::IAdd(a, b) | Inst::IMul(a, b) => self.value_is_nonneg(*a) && self.value_is_nonneg(*b),
            Inst::LShl(a, b) => {
                self.value_is_nonneg(*a) && self.value_is_nonneg_iconst(*b)
            }
            Inst::LShr(a, _) => self.value_is_nonneg(*a),
            Inst::ISub(a, b) => self.value_is_nonneg(*a) && self.value_is_nonneg(*b),
            Inst::IAnd(l, _) => self.value_is_nonneg(*l),
            Inst::ZExtI64(_) => true,
            _ => false,
        }
    }

    fn update_nonneg_sym(&mut self, sym: Symbol, op: AssignOp, rhs: ValueId, result: ValueId) {
        match op {
            AssignOp::Eq => {
                if self.value_is_iconst(result, 0)
                    || self.value_is_nonneg_rem_result(result)
                    || self.value_is_nonneg(result)
                {
                    self.mut_nonneg_syms.insert(sym);
                } else {
                    self.mut_nonneg_syms.remove(&sym);
                }
                if let Some(b) = self.value_excl_bound(result) {
                    self.mut_excl_bound.insert(sym, b);
                } else if let Some(m) = self.rem_result_modulus(result) {
                    self.mut_excl_bound.insert(sym, m);
                } else {
                    self.mut_excl_bound.remove(&sym);
                }
            }
            AssignOp::PlusEq => {
                if !self.value_is_nonneg_iconst(rhs) {
                    self.mut_nonneg_syms.remove(&sym);
                    self.mut_excl_bound.remove(&sym);
                } else if self.mut_nonneg_syms.contains(&sym) && self.value_is_nonneg(result) {
                    self.mut_nonneg_syms.insert(sym);
                    self.mut_excl_bound.remove(&sym);
                }
            }
            AssignOp::MinusEq
            | AssignOp::StarEq
            | AssignOp::SlashEq
            | AssignOp::PercentEq => {
                self.mut_nonneg_syms.remove(&sym);
                self.mut_excl_bound.remove(&sym);
            }
        }
    }

    fn rem_result_modulus(&self, v: ValueId) -> Option<i64> {
        let def = self.b.func.value(v).def?;
        match self.b.func.inst(def) {
            Inst::URem(_, d) | Inst::IRem(_, d) => self.positive_iconst(*d),
            _ => None,
        }
    }

    fn update_positive_sym(&mut self, sym: Symbol, op: AssignOp, result: ValueId) {
        match op {
            AssignOp::Eq => {
                if self.value_is_strictly_positive(result) {
                    self.mut_positive_syms.insert(sym);
                } else {
                    self.mut_positive_syms.remove(&sym);
                }
            }
            AssignOp::PlusEq => {
                if !self.mut_positive_syms.contains(&sym) {
                    return;
                }
                if self.value_is_strictly_positive(result) {
                    self.mut_positive_syms.insert(sym);
                } else {
                    self.mut_positive_syms.remove(&sym);
                }
            }
            AssignOp::MinusEq
            | AssignOp::StarEq
            | AssignOp::SlashEq
            | AssignOp::PercentEq => {
                self.mut_positive_syms.remove(&sym);
            }
        }
    }

    fn lower_int_mul(&mut self, l: ValueId, r: ValueId) -> ValueId {
        let strength = |this: &mut Self, var: ValueId, mult: ValueId| -> Option<ValueId> {
            if !this.value_is_nonneg(var) {
                return None;
            }
            let (shift, plus_one) = this.iconst_shift_mul_form(mult)?;
            let sh = this.b.iconst(i64::from(shift));
            let shifted = this.b.push_value(Inst::LShl(var, sh));
            Some(if plus_one {
                this.b.push_value(Inst::IAdd(shifted, var))
            } else {
                this.b.push_value(Inst::ISub(shifted, var))
            })
        };
        if let Some(v) = strength(self, l, r).or_else(|| strength(self, r, l)) {
            return v;
        }
        self.b.push_value(Inst::IMul(l, r))
    }

    fn lower_int_div(&mut self, l: ValueId, r: ValueId) -> ValueId {
        if let Some(shift) = self.iconst_pow2_shift(r) {
            if self.value_is_nonneg(l) {
                let sh = self.b.iconst(i64::from(shift));
                return self.b.push_value(Inst::LShr(l, sh));
            }
        }
        if self.value_is_nonneg(l) && self.value_is_strictly_positive(r) {
            return self.b.push_value(Inst::UDiv(l, r));
        }
        self.b.push_value(Inst::IDiv(l, r))
    }

    fn lower_int_rem(&mut self, l: ValueId, r: ValueId) -> ValueId {
        if let Some(shift) = self.iconst_pow2_shift(r) {
            if self.value_is_nonneg(l) {
                let mask = (1i64 << shift) - 1;
                let m = self.b.iconst(mask);
                return self.b.push_value(Inst::IAnd(l, m));
            }
        }
        // `(a + b) % m` with a,b ∈ [0, m) → at most one subtract (hash second step).
        if let Some(m) = self.positive_iconst(r) {
            if let Some(def) = self.b.func.value(l).def {
                if let Inst::IAdd(a, b) = self.b.func.inst(def) {
                    let ab = self.value_excl_bound(*a);
                    let bb = self.value_excl_bound(*b);
                    if ab.is_some_and(|x| x <= m) && bb.is_some_and(|x| x <= m) {
                        let ge = self.b.push_value(Inst::ICmp(CmpOp::Ge, l, r));
                        let z = self.b.push_value(Inst::ZExtI64(ge));
                        let adj = self.b.push_value(Inst::IMul(z, r));
                        let t = self.b.push_value(Inst::ISub(l, adj));
                        self.value_excl_bound_map.insert(t, m);
                        return t;
                    }
                }
            }
        }
        // `(x * k) % m` with x ∈ [0, m) and small k → k conditional subtracts.
        if let Some(m) = self.positive_iconst(r) {
            if let Some((base, k)) = self.value_as_small_factor_mul(l) {
                let bound = self.value_excl_bound(base);
                if (2..=8).contains(&k) && bound.is_some_and(|b| b <= m) {
                    let mut t = l;
                    for _ in 0..k {
                        let ge = self.b.push_value(Inst::ICmp(CmpOp::Ge, t, r));
                        let z = self.b.push_value(Inst::ZExtI64(ge));
                        let adj = self.b.push_value(Inst::IMul(z, r));
                        t = self.b.push_value(Inst::ISub(t, adj));
                    }
                    self.value_excl_bound_map.insert(t, m);
                    return t;
                }
            }
        }
        if self.value_is_nonneg(l) && self.value_is_strictly_positive(r) {
            return self.b.push_value(Inst::URem(l, r));
        }
        // Both operands non-negative and divisor not literal 0 (Euclidean gcd path).
        if self.value_is_nonneg(l)
            && self.value_is_nonneg(r)
            && !self.value_is_iconst(r, 0)
        {
            return self.b.push_value(Inst::URem(l, r));
        }
        self.b.push_value(Inst::IRem(l, r))
    }

    /// Exclusive upper bound for a value if known (`v ∈ [0, bound)`).
    fn value_excl_bound(&self, v: ValueId) -> Option<i64> {
        if let Some(&b) = self.value_excl_bound_map.get(&v) {
            return Some(b);
        }
        if let Some(n) = self.iconst_value(v) {
            if n >= 0 {
                return Some(n.saturating_add(1));
            }
            return None;
        }
        for frame in self.loop_carried.iter().rev() {
            for c in frame {
                if c.current == v {
                    if let Some(b) = c.excl_bound {
                        return Some(b);
                    }
                    if let Some(&b) = self.mut_excl_bound.get(&c.sym) {
                        return Some(b);
                    }
                }
            }
        }
        for (sym, local) in self.locals.iter() {
            let matches = match local {
                Local::MutSsa(x) | Local::Ssa(x) => *x == v,
                Local::Slot(_) => false,
            };
            if matches {
                if let Some(&b) = self.mut_excl_bound.get(sym) {
                    return Some(b);
                }
            }
        }
        let def = self.b.func.value(v).def?;
        match self.b.func.inst(def) {
            Inst::URem(_, d) | Inst::IRem(_, d) => self.positive_iconst(*d),
            _ => None,
        }
    }

    fn iconst_value(&self, v: ValueId) -> Option<i64> {
        let def = self.b.func.value(v).def?;
        match self.b.func.inst(def) {
            Inst::IConst(n) => Some(*n),
            _ => None,
        }
    }

    /// Match `x*k` for small k, including strength-reduced `x + (x<<s)` / ` (x<<s) - x` forms.
    fn value_as_small_factor_mul(&self, v: ValueId) -> Option<(ValueId, i64)> {
        let def = self.b.func.value(v).def?;
        match self.b.func.inst(def) {
            Inst::IMul(a, b) => {
                if let Some(k) = self.positive_iconst(*a) {
                    return Some((*b, k));
                }
                if let Some(k) = self.positive_iconst(*b) {
                    return Some((*a, k));
                }
                None
            }
            Inst::IAdd(a, b) => {
                if let Some((x, s)) = self.value_as_shl_const(*a) {
                    if x == *b {
                        return Some((x, (1i64 << s) + 1));
                    }
                }
                if let Some((x, s)) = self.value_as_shl_const(*b) {
                    if x == *a {
                        return Some((x, (1i64 << s) + 1));
                    }
                }
                None
            }
            Inst::ISub(a, b) => {
                if let Some((x, s)) = self.value_as_shl_const(*a) {
                    if x == *b {
                        return Some((x, (1i64 << s) - 1));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn value_as_shl_const(&self, v: ValueId) -> Option<(ValueId, u32)> {
        let def = self.b.func.value(v).def?;
        let Inst::LShl(a, b) = self.b.func.inst(def) else {
            return None;
        };
        let sh = self.positive_iconst(*b)?;
        if sh > 0 && sh < 63 {
            Some((*a, sh as u32))
        } else {
            None
        }
    }

    fn loop_is_linear(&self, body: &[Stmt<'_>]) -> bool {
        loop_carried_is_linear(body) && self.loops.iter().all(|f| f.linear_carried)
    }

    fn loop_uses_alloca_carried(&self) -> bool {
        self.loop_carried_linear
            .last()
            .is_some_and(|&linear| !linear)
    }

    fn loop_guard_eligible(&self, guard: LoopExitGuard) -> bool {
        let counter_ok = |sym: Symbol| {
            matches!(
                self.locals.get(&sym),
                Some(Local::MutSsa(_) | Local::Slot(_))
            )
        };
        match guard {
            LoopExitGuard::CountedGe { counter, bound }
            | LoopExitGuard::CountedGt { counter, bound } => {
                counter_ok(counter) && matches!(self.locals.get(&bound), Some(Local::Ssa(_)))
            }
            LoopExitGuard::CountedGeLit { counter, .. } => counter_ok(counter),
            LoopExitGuard::Zero { counter } => counter_ok(counter),
            LoopExitGuard::SquareGt { counter, bound } => {
                counter_ok(counter)
                    && matches!(
                        self.locals.get(&bound),
                        Some(Local::Ssa(_) | Local::MutSsa(_) | Local::Slot(_))
                    )
            }
            LoopExitGuard::RemZero {
                dividend,
                divisor,
                clear_sym,
                ..
            } => {
                matches!(
                    self.locals.get(&dividend),
                    Some(Local::Ssa(_) | Local::MutSsa(_) | Local::Slot(_))
                ) && counter_ok(divisor)
                    && matches!(
                        self.locals.get(&clear_sym),
                        Some(Local::MutSsa(_) | Local::Slot(_))
                    )
            }
        }
    }

    fn guard_continue_cond(&mut self, guard: LoopExitGuard) -> ValueId {
        match guard {
            LoopExitGuard::CountedGe { counter, bound } => {
                let counter_val = self.load_sym(counter);
                let bound_val = self.load_sym(bound);
                self.b
                    .push_value(Inst::ICmp(CmpOp::Lt, counter_val, bound_val))
            }
            LoopExitGuard::CountedGeLit { counter, bound } => {
                let counter_val = self.load_sym(counter);
                let bound_val = self.b.iconst(bound);
                self.b
                    .push_value(Inst::ICmp(CmpOp::Lt, counter_val, bound_val))
            }
            LoopExitGuard::CountedGt { counter, bound } => {
                let counter_val = self.load_sym(counter);
                let bound_val = self.load_sym(bound);
                self.b
                    .push_value(Inst::ICmp(CmpOp::Le, counter_val, bound_val))
            }
            LoopExitGuard::Zero { counter } => {
                let counter_val = self.load_sym(counter);
                let zero = self.b.iconst(0);
                self.b
                    .push_value(Inst::ICmp(CmpOp::Ne, counter_val, zero))
            }
            LoopExitGuard::SquareGt { counter, bound } => {
                let counter_val = self.load_sym(counter);
                let bound_val = self.load_sym(bound);
                let sq = self.b.push_value(Inst::IMul(counter_val, counter_val));
                self.b.push_value(Inst::ICmp(CmpOp::Le, sq, bound_val))
            }
            LoopExitGuard::RemZero {
                dividend,
                divisor,
                ..
            } => {
                let d = self.load_sym(dividend);
                let r = self.load_sym(divisor);
                let rem = self.lower_int_rem(d, r);
                let zero = self.b.iconst(0);
                self.b.push_value(Inst::ICmp(CmpOp::Ne, rem, zero))
            }
        }
    }
}
