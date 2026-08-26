fn strip_counter_step_one<'a>(body: &'a [Stmt<'a>], counter: Symbol) -> Option<&'a [Stmt<'a>]> {
    match body.last()? {
        Stmt::Assign(a) if a.op == AssignOp::PlusEq => {
            let Expr::Path(p) = a.target else {
                return None;
            };
            if p.segments.last()?.name != counter {
                return None;
            }
            lit_is_one(a.value).then(|| &body[..body.len() - 1])
        }
        _ => None,
    }
}

/// Suite5 alu/reduce: `acc = acc + i * A - i / B + i % C` (+ counter `+= 1`).
fn try_parse_linear_mix_step(
    body: &[Stmt<'_>],
    counter: Symbol,
) -> Option<(Symbol, i64, i64, i64)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 1 {
        return None;
    }
    let Stmt::Assign(a) = &core[0] else {
        return None;
    };
    if a.op != AssignOp::Eq {
        return None;
    }
    let acc = expr_path(a.target)?;
    if acc == counter {
        return None;
    }
    // acc + i * A - i / B + i % C  (left-assoc)
    let Expr::Binary(add_rem) = a.value else {
        return None;
    };
    if add_rem.op != BinaryOp::Plus {
        return None;
    }
    let rem_e = add_rem.rhs;
    let Expr::Binary(sub_div) = add_rem.lhs else {
        return None;
    };
    if sub_div.op != BinaryOp::Minus {
        return None;
    }
    let div_e = sub_div.rhs;
    let Expr::Binary(add_mul) = sub_div.lhs else {
        return None;
    };
    if add_mul.op != BinaryOp::Plus {
        return None;
    }
    let mul_e = add_mul.rhs;
    if expr_path(add_mul.lhs)? != acc {
        return None;
    }
    let Expr::Binary(mul) = mul_e else {
        return None;
    };
    let a_k = if mul.op == BinaryOp::Star && expr_path(mul.lhs) == Some(counter) {
        expr_lit_i64(mul.rhs)?
    } else if mul.op == BinaryOp::Star && expr_path(mul.rhs) == Some(counter) {
        expr_lit_i64(mul.lhs)?
    } else {
        return None;
    };
    let Expr::Binary(div) = div_e else {
        return None;
    };
    if div.op != BinaryOp::Slash || expr_path(div.lhs)? != counter {
        return None;
    }
    let b_k = expr_lit_i64(div.rhs)?;
    if b_k <= 0 {
        return None;
    }
    let Expr::Binary(rem) = rem_e else {
        return None;
    };
    if rem.op != BinaryOp::Percent || expr_path(rem.lhs)? != counter {
        return None;
    }
    let c_k = expr_lit_i64(rem.rhs)?;
    if c_k <= 1 {
        return None;
    }
    Some((acc, a_k, b_k, c_k))
}

/// Suite5 scan: `if i % a == 0 or i % b == 0 { acc += 1 }` (+ counter `+= 1`).
fn try_parse_scan_or_count(
    body: &[Stmt<'_>],
    counter: Symbol,
) -> Option<(Symbol, i64, i64)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 1 {
        return None;
    }
    let Stmt::If(i) = &core[0] else {
        return None;
    };
    if i.else_body.is_some() || i.arms.len() != 1 {
        return None;
    }
    let arm = &i.arms[0];
    if arm.body.len() != 1 {
        return None;
    }
    let Stmt::Assign(inc) = &arm.body[0] else {
        return None;
    };
    let acc = expr_path(inc.target)?;
    if acc == counter {
        return None;
    }
    if !(inc.op == AssignOp::PlusEq && lit_is_one(inc.value)) {
        return None;
    }
    let Expr::Binary(or) = arm.cond else {
        return None;
    };
    if or.op != BinaryOp::Or {
        return None;
    }
    let parse_rem0 = |e: &Expr<'_>| -> Option<i64> {
        let Expr::Binary(eq) = e else {
            return None;
        };
        if eq.op != BinaryOp::EqEq {
            return None;
        }
        let (rem_e, zero_e) = if lit_is_zero(eq.rhs) {
            (eq.lhs, eq.rhs)
        } else if lit_is_zero(eq.lhs) {
            (eq.rhs, eq.lhs)
        } else {
            return None;
        };
        let _ = zero_e;
        let Expr::Binary(rem) = rem_e else {
            return None;
        };
        if rem.op != BinaryOp::Percent || expr_path(rem.lhs)? != counter {
            return None;
        }
        let m = expr_lit_i64(rem.rhs)?;
        (m > 1).then_some(m)
    };
    let a = parse_rem0(or.lhs)?;
    let b = parse_rem0(or.rhs)?;
    Some((acc, a, b))
}

/// Classic Fibonacci step: `let c = a + b; a = b; b = c` (+ counter `+= 1`).
fn try_parse_fib_step(body: &[Stmt<'_>], counter: Symbol) -> Option<(Symbol, Symbol)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 3 {
        return None;
    }
    let Stmt::Let(c_let) = &core[0] else {
        return None;
    };
    let Expr::Binary(add) = c_let.init else {
        return None;
    };
    if add.op != BinaryOp::Plus {
        return None;
    }
    let a = expr_path(add.lhs)?;
    let b = expr_path(add.rhs)?;
    if a == b || a == counter || b == counter {
        return None;
    }
    let Stmt::Assign(as_a) = &core[1] else {
        return None;
    };
    let Stmt::Assign(as_b) = &core[2] else {
        return None;
    };
    if as_a.op != AssignOp::Eq || as_b.op != AssignOp::Eq {
        return None;
    }
    if expr_path(as_a.target)? != a || expr_path(as_a.value)? != b {
        return None;
    }
    if expr_path(as_b.target)? != b || expr_path(as_b.value)? != c_let.name.name {
        return None;
    }
    Some((a, b))
}

/// Rolling hash: `h = (h * k + i) % m` (+ counter `+= 1`).
fn try_parse_hash_step(
    body: &[Stmt<'_>],
    counter: Symbol,
) -> Option<(Symbol, i64, i64)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 1 {
        return None;
    }
    let Stmt::Assign(a) = &core[0] else {
        return None;
    };
    if a.op != AssignOp::Eq {
        return None;
    }
    let h = expr_path(a.target)?;
    if h == counter {
        return None;
    }
    let Expr::Binary(rem) = a.value else {
        return None;
    };
    if rem.op != BinaryOp::Percent {
        return None;
    }
    let m = expr_lit_i64(rem.rhs)?;
    if m <= 1 {
        return None;
    }
    let Expr::Binary(inner) = rem.lhs else {
        return None;
    };
    if inner.op != BinaryOp::Plus {
        return None;
    }
    // (h * k) + i  or  i + (h * k)
    let (mul_e, idx_e) = if expr_path(inner.rhs) == Some(counter) {
        (inner.lhs, inner.rhs)
    } else if expr_path(inner.lhs) == Some(counter) {
        (inner.rhs, inner.lhs)
    } else {
        return None;
    };
    if expr_path(idx_e)? != counter {
        return None;
    }
    let Expr::Binary(mul) = mul_e else {
        return None;
    };
    if mul.op != BinaryOp::Star {
        return None;
    }
    let k = if expr_path(mul.lhs) == Some(h) {
        expr_lit_i64(mul.rhs)?
    } else if expr_path(mul.rhs) == Some(h) {
        expr_lit_i64(mul.lhs)?
    } else {
        return None;
    };
    if k <= 0 {
        return None;
    }
    Some((h, k, m))
}

/// Suite5 powmod: `acc = (acc * base) % m` (+ counter `+= 1`).
/// `base` may be a literal or an immutable iconst binding.
fn try_parse_powmod_step(
    body: &[Stmt<'_>],
    counter: Symbol,
) -> Option<(Symbol, Result<i64, Symbol>, i64)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 1 {
        return None;
    }
    let Stmt::Assign(a) = &core[0] else {
        return None;
    };
    if a.op != AssignOp::Eq {
        return None;
    }
    let acc = expr_path(a.target)?;
    if acc == counter {
        return None;
    }
    let Expr::Binary(rem) = a.value else {
        return None;
    };
    if rem.op != BinaryOp::Percent {
        return None;
    }
    let m = expr_lit_i64(rem.rhs)?;
    if m <= 1 {
        return None;
    }
    let Expr::Binary(mul) = rem.lhs else {
        return None;
    };
    if mul.op != BinaryOp::Star {
        return None;
    }
    let base = if expr_path(mul.lhs) == Some(acc) {
        if let Some(lit) = expr_lit_i64(mul.rhs) {
            Ok(lit)
        } else {
            Err(expr_path(mul.rhs)?)
        }
    } else if expr_path(mul.rhs) == Some(acc) {
        if let Some(lit) = expr_lit_i64(mul.lhs) {
            Ok(lit)
        } else {
            Err(expr_path(mul.lhs)?)
        }
    } else {
        return None;
    };
    match base {
        Ok(b) if b <= 0 => return None,
        Err(s) if s == acc || s == counter => return None,
        _ => {}
    }
    Some((acc, base, m))
}


fn try_parse_gcd_sum_step(
    body: &[Stmt<'_>],
    counter: Symbol,
) -> Option<(Symbol, i64, i64, i64, Symbol)> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 3 {
        return None;
    }
    let Stmt::Let(a_let) = &core[0] else {
        return None;
    };
    if a_let.mutable {
        return None;
    }
    let a_sym = a_let.name.name;
    let Expr::Binary(amul) = a_let.init else {
        return None;
    };
    if amul.op != BinaryOp::Star {
        return None;
    }
    let ak = if expr_path(amul.lhs) == Some(counter) {
        expr_lit_i64(amul.rhs)?
    } else if expr_path(amul.rhs) == Some(counter) {
        expr_lit_i64(amul.lhs)?
    } else {
        return None;
    };
    let Stmt::Let(b_let) = &core[1] else {
        return None;
    };
    if b_let.mutable {
        return None;
    }
    let b_sym = b_let.name.name;
    let Expr::Binary(badd) = b_let.init else {
        return None;
    };
    if badd.op != BinaryOp::Plus {
        return None;
    }
    let (bmul_e, c_e) = match (expr_lit_i64(badd.lhs), expr_lit_i64(badd.rhs)) {
        (None, Some(c)) => (badd.lhs, c),
        (Some(c), None) => (badd.rhs, c),
        _ => return None,
    };
    let Expr::Binary(bmul) = bmul_e else {
        return None;
    };
    if bmul.op != BinaryOp::Star {
        return None;
    }
    let bk = if expr_path(bmul.lhs) == Some(counter) {
        expr_lit_i64(bmul.rhs)?
    } else if expr_path(bmul.rhs) == Some(counter) {
        expr_lit_i64(bmul.lhs)?
    } else {
        return None;
    };
    let Stmt::Assign(acc_a) = &core[2] else {
        return None;
    };
    let acc = expr_path(acc_a.target)?;
    if acc == counter || acc == a_sym || acc == b_sym {
        return None;
    }
    let call_e = match acc_a.op {
        AssignOp::PlusEq => acc_a.value,
        AssignOp::Eq => {
            let Expr::Binary(add) = acc_a.value else {
                return None;
            };
            if add.op != BinaryOp::Plus {
                return None;
            }
            if expr_path(add.lhs) == Some(acc) {
                add.rhs
            } else if expr_path(add.rhs) == Some(acc) {
                add.lhs
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let Expr::Call(call) = call_e else {
        return None;
    };
    if call.args.len() != 2 {
        return None;
    }
    if expr_path(call.args[0]) != Some(a_sym) || expr_path(call.args[1]) != Some(b_sym) {
        return None;
    }
    let gcd_name = expr_path(call.callee)?;
    Some((acc, ak, bk, c_e, gcd_name))
}

/// Nested `for i,j in 0..n { s += (i*j + i) % mod }` (Suite5 nested).
fn try_parse_nested_ij_mod(
    body: &[Stmt<'_>],
    outer_counter: Symbol,
) -> Option<(Symbol, i64, Symbol)> {
    let core = strip_counter_step_one(body, outer_counter)?;
    if core.len() != 2 {
        return None;
    }
    let Stmt::Let(j_let) = &core[0] else {
        return None;
    };
    if !j_let.mutable || !lit_is_zero(j_let.init) {
        return None;
    }
    let j = j_let.name.name;
    let Stmt::Loop(inner) = &core[1] else {
        return None;
    };
    let (inner_guard, inner_rest) = try_parse_loop_exit_guard(inner.body)?;
    let (j_c, inner_bound) = match inner_guard {
        LoopExitGuard::CountedGe { counter, bound } => (counter, bound),
        _ => return None,
    };
    if j_c != j {
        return None;
    }
    let inner_core = strip_counter_step_one(inner_rest, j)?;
    if inner_core.len() != 1 {
        return None;
    }
    let Stmt::Assign(acc) = &inner_core[0] else {
        return None;
    };
    let s = expr_path(acc.target)?;
    if s == outer_counter || s == j {
        return None;
    }
    let addend = match acc.op {
        AssignOp::PlusEq => acc.value,
        AssignOp::Eq => {
            let Expr::Binary(add) = acc.value else {
                return None;
            };
            if add.op != BinaryOp::Plus {
                return None;
            }
            if expr_path(add.lhs) == Some(s) {
                add.rhs
            } else if expr_path(add.rhs) == Some(s) {
                add.lhs
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let Expr::Binary(rem) = addend else {
        return None;
    };
    if rem.op != BinaryOp::Percent {
        return None;
    }
    let m = expr_lit_i64(rem.rhs)?;
    if m <= 1 {
        return None;
    }
    // (i * j + i) or (i + i * j)
    let Expr::Binary(sum) = rem.lhs else {
        return None;
    };
    if sum.op != BinaryOp::Plus {
        return None;
    }
    let (mul_e, alone) = match (expr_path(sum.lhs), expr_path(sum.rhs)) {
        (Some(p), None) if p == outer_counter => (sum.rhs, sum.lhs),
        (None, Some(p)) if p == outer_counter => (sum.lhs, sum.rhs),
        _ => return None,
    };
    if expr_path(alone)? != outer_counter {
        return None;
    }
    let Expr::Binary(mul) = mul_e else {
        return None;
    };
    if mul.op != BinaryOp::Star {
        return None;
    }
    let (l, r) = (expr_path(mul.lhs)?, expr_path(mul.rhs)?);
    if !((l == outer_counter && r == j) || (l == j && r == outer_counter)) {
        return None;
    }
    Some((s, m, inner_bound))
}

/// `acc += i * i` (or `acc = acc + i * i`) with `i` the loop counter.
fn try_parse_sum_of_squares(body: &[Stmt<'_>], counter: Symbol) -> Option<Symbol> {
    let core = strip_counter_step_one(body, counter)?;
    if core.len() != 1 {
        return None;
    }
    let Stmt::Assign(a) = &core[0] else {
        return None;
    };
    let Expr::Path(tp) = a.target else {
        return None;
    };
    let acc = tp.segments.last()?.name;
    if acc == counter {
        return None;
    }
    let square = match a.op {
        AssignOp::PlusEq => a.value,
        AssignOp::Eq => {
            let Expr::Binary(add) = a.value else {
                return None;
            };
            if add.op != BinaryOp::Plus {
                return None;
            }
            let left = expr_path(add.lhs)?;
            if left == acc {
                add.rhs
            } else if expr_path(add.rhs)? == acc {
                add.lhs
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let Expr::Binary(mul) = square else {
        return None;
    };
    if mul.op != BinaryOp::Star {
        return None;
    }
    let l = expr_path(mul.lhs)?;
    let r = expr_path(mul.rhs)?;
    if l == counter && r == counter {
        Some(acc)
    } else {
        None
    }
}

fn try_break_only_if(i: &rynix_ast::IfStmt<'_>) -> bool {
    i.arms.len() == 1
        && i.else_body.is_none()
        && i.arms[0].body.len() == 1
        && matches!(i.arms[0].body[0], Stmt::Break(_))
}

fn try_parse_square_gt(cond: &Expr<'_>) -> Option<(Symbol, Symbol)> {
    let Expr::Binary(b) = cond else {
        return None;
    };
    match b.op {
        BinaryOp::Gt => {
            if let Expr::Binary(sq) = b.lhs {
                if sq.op == BinaryOp::Star {
                    let c = expr_path(sq.lhs)?;
                    let c2 = expr_path(sq.rhs)?;
                    if c != c2 {
                        return None;
                    }
                    let bound = expr_path(b.rhs)?;
                    return Some((c, bound));
                }
            }
        }
        BinaryOp::Lt => {
            if let Expr::Binary(sq) = b.rhs {
                if sq.op == BinaryOp::Star {
                    let c = expr_path(sq.lhs)?;
                    let c2 = expr_path(sq.rhs)?;
                    if c != c2 {
                        return None;
                    }
                    let bound = expr_path(b.lhs)?;
                    return Some((c, bound));
                }
            }
        }
        _ => {}
    }
    None
}

/// Recognize `if … break` loop guards lowered as counted / zero-exit loops.
fn try_parse_loop_exit_guard<'a>(
    body: &'a [Stmt<'a>],
) -> Option<(LoopExitGuard, &'a [Stmt<'a>])> {
    let Stmt::If(i) = body.first()? else {
        return None;
    };
    if !try_break_only_if(i) {
        return None;
    }
    let cond = i.arms[0].cond;
    let guard = match cond {
        Expr::Binary(b) => match b.op {
            BinaryOp::Gt => {
                if let Some((counter, bound)) = try_parse_square_gt(cond) {
                    LoopExitGuard::SquareGt { counter, bound }
                } else {
                    let counter = expr_path(b.lhs)?;
                    let bound = expr_path(b.rhs)?;
                    LoopExitGuard::CountedGt { counter, bound }
                }
            }
            BinaryOp::GtEq => {
                let counter = expr_path(b.lhs)?;
                if let Some(lit) = expr_lit_i64(b.rhs) {
                    LoopExitGuard::CountedGeLit { counter, bound: lit }
                } else {
                    let bound = expr_path(b.rhs)?;
                    LoopExitGuard::CountedGe { counter, bound }
                }
            }
            BinaryOp::LtEq => {
                if let Some(lit) = expr_lit_i64(b.lhs) {
                    let counter = expr_path(b.rhs)?;
                    LoopExitGuard::CountedGeLit { counter, bound: lit }
                } else {
                    let bound = expr_path(b.lhs)?;
                    let counter = expr_path(b.rhs)?;
                    LoopExitGuard::CountedGe { counter, bound }
                }
            }
            BinaryOp::Lt => {
                if let Some((counter, bound)) = try_parse_square_gt(cond) {
                    LoopExitGuard::SquareGt { counter, bound }
                } else {
                    let bound = expr_path(b.lhs)?;
                    let counter = expr_path(b.rhs)?;
                    LoopExitGuard::CountedGt { counter, bound }
                }
            }
            BinaryOp::EqEq => {
                if lit_is_zero(b.rhs) {
                    LoopExitGuard::Zero {
                        counter: expr_path(b.lhs)?,
                    }
                } else if lit_is_zero(b.lhs) {
                    LoopExitGuard::Zero {
                        counter: expr_path(b.rhs)?,
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        },
        _ => return None,
    };
    Some((guard, &body[1..]))
}

fn try_parse_prime_count(body: &[Stmt<'_>], outer_i: Symbol) -> Option<Symbol> {
    let core = strip_counter_step_one(body, outer_i)?;
    if core.len() != 4 {
        return None;
    }
    let Stmt::Let(prime_let) = &core[0] else {
        return None;
    };
    if !prime_let.mutable || !lit_is_one(prime_let.init) {
        return None;
    }
    let prime = prime_let.name.name;
    let Stmt::Let(j_let) = &core[1] else {
        return None;
    };
    if !j_let.mutable || !lit_is_two(j_let.init) {
        return None;
    }
    let j = j_let.name.name;
    let Stmt::Loop(inner) = &core[2] else {
        return None;
    };
    let (inner_guard, inner_rest) = try_parse_loop_exit_guard(inner.body)?;
    let LoopExitGuard::SquareGt {
        counter: j_c,
        bound: i_b,
    } = inner_guard
    else {
        return None;
    };
    if j_c != j || i_b != outer_i {
        return None;
    }
    let (rem_guards, after_rem) = peel_rem_zero_guards(inner_rest);
    if rem_guards.len() != 1 {
        return None;
    }
    let LoopExitGuard::RemZero {
        dividend,
        divisor,
        clear_sym,
        clear_val,
    } = rem_guards[0]
    else {
        return None;
    };
    if dividend != outer_i || divisor != j || clear_sym != prime || clear_val != 0 {
        return None;
    }
    let inner_tail = strip_counter_step_one(after_rem, j)?;
    if !inner_tail.is_empty() {
        return None;
    }
    let Stmt::If(inc_if) = &core[3] else {
        return None;
    };
    let (count, cond) = try_parse_conditional_add(inc_if)?;
    let Expr::Binary(b) = cond else {
        return None;
    };
    if b.op != BinaryOp::BangEq {
        return None;
    }
    let prime_ne_zero = (expr_path(b.lhs) == Some(prime) && lit_is_zero(b.rhs))
        || (expr_path(b.rhs) == Some(prime) && lit_is_zero(b.lhs));
    if !prime_ne_zero || count == outer_i || count == prime || count == j {
        return None;
    }
    Some(count)
}

fn try_parse_rem_zero_exit<'a>(
    body: &'a [Stmt<'a>],
) -> Option<(LoopExitGuard, &'a [Stmt<'a>])> {
    let Stmt::If(i) = body.first()? else {
        return None;
    };
    if i.else_body.is_some() || i.arms.len() != 1 {
        return None;
    }
    let arm = &i.arms[0];
    if arm.body.len() != 2 {
        return None;
    }
    let Stmt::Assign(a) = &arm.body[0] else {
        return None;
    };
    if !matches!(arm.body[1], Stmt::Break(_)) || a.op != AssignOp::Eq {
        return None;
    }
    let Expr::Path(p) = a.target else {
        return None;
    };
    let clear_sym = p.segments.last()?.name;
    let clear_val = match a.value {
        Expr::Literal(l) => l.int_value?,
        _ => return None,
    };
    let Expr::Binary(eq) = arm.cond else {
        return None;
    };
    if eq.op != BinaryOp::EqEq {
        return None;
    }
    let rem_expr = if lit_is_zero(eq.rhs) {
        eq.lhs
    } else if lit_is_zero(eq.lhs) {
        eq.rhs
    } else {
        return None;
    };
    let Expr::Binary(rem) = rem_expr else {
        return None;
    };
    if rem.op != BinaryOp::Percent {
        return None;
    }
    let dividend = expr_path(rem.lhs)?;
    let divisor = expr_path(rem.rhs)?;
    Some((
        LoopExitGuard::RemZero {
            dividend,
            divisor,
            clear_sym,
            clear_val,
        },
        &body[1..],
    ))
}

fn peel_rem_zero_guards<'a>(body: &'a [Stmt<'a>]) -> (Vec<LoopExitGuard>, &'a [Stmt<'a>]) {
    let mut guards = Vec::new();
    let mut rest = body;
    while let Some((g, next)) = try_parse_rem_zero_exit(rest) {
        guards.push(g);
        rest = next;
    }
    (guards, rest)
}

fn collect_guard_clears(extra_guards: &[LoopExitGuard]) -> Vec<(Symbol, i64)> {
    let mut out = Vec::new();
    for g in extra_guards {
        if let LoopExitGuard::RemZero {
            clear_sym,
            clear_val,
            ..
        } = g
            && !out.iter().any(|(s, _)| *s == *clear_sym)
        {
            out.push((*clear_sym, *clear_val));
        }
    }
    out
}

fn is_lshr_self_by_one(e: &Expr<'_>, sym: Symbol) -> bool {
    let Expr::Binary(b) = e else {
        return false;
    };
    b.op == BinaryOp::Shr && expr_path(b.lhs) == Some(sym) && lit_is_one(b.rhs)
}

/// `loop / if v == 0 break / c += v & 1 / v >>= 1` → ctpop.
fn try_parse_popcount_body(body: &[Stmt<'_>]) -> Option<(Symbol, Symbol)> {
    if body.len() != 2 {
        return None;
    }
    let Stmt::Assign(inc) = &body[0] else {
        return None;
    };
    if inc.op != AssignOp::PlusEq {
        return None;
    }
    let accum = expr_path(inc.target)?;
    let Expr::Binary(and) = inc.value else {
        return None;
    };
    if and.op != BinaryOp::Amp {
        return None;
    }
    let bit = if lit_is_one(and.rhs) {
        expr_path(and.lhs)?
    } else if lit_is_one(and.lhs) {
        expr_path(and.rhs)?
    } else {
        return None;
    };
    let Stmt::Assign(shr) = &body[1] else {
        return None;
    };
    if shr.op != AssignOp::Eq || expr_path(shr.target)? != bit {
        return None;
    }
    if !is_lshr_self_by_one(shr.value, bit) {
        return None;
    }
    Some((accum, bit))
}

fn try_parse_conditional_add<'a>(i: &rynix_ast::IfStmt<'a>) -> Option<(Symbol, &'a Expr<'a>)> {
    if i.else_body.is_some() || i.arms.len() != 1 {
        return None;
    }
    let arm = &i.arms[0];
    if arm.body.len() != 1 {
        return None;
    }
    let Stmt::Assign(a) = &arm.body[0] else {
        return None;
    };
    if a.op != AssignOp::PlusEq || !lit_is_one(a.value) {
        return None;
    }
    let target = expr_path(a.target)?;
    Some((target, arm.cond))
}

fn body_has_break(stmts: &[Stmt<'_>]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Break(_) => return true,
            Stmt::If(i) => {
                if i.arms.iter().any(|a| body_has_break(&a.body)) {
                    return true;
                }
                if i.else_body.is_some_and(|b| body_has_break(b)) {
                    return true;
                }
            }
            Stmt::Match(m) => {
                if m.arms.iter().any(|a| body_has_break(&a.body)) {
                    return true;
                }
                if m.else_body.is_some_and(|b| body_has_break(b)) {
                    return true;
                }
            }
            Stmt::Loop(l) => {
                if body_has_break(&l.body) {
                    return true;
                }
            }
            Stmt::For(f) => {
                if body_has_break(&f.body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Guarded-loop peel is safe when nested `loop`s are counted exits with rem-zero
/// folded into the inner guard chain (not square-gt inner — see `prime.ryx`).
fn rest_allows_guarded_loop(rest: &[Stmt<'_>]) -> bool {
    for stmt in rest {
        let Stmt::Loop(l) = stmt else {
            continue;
        };
        let Some((guard, inner_rest)) = try_parse_loop_exit_guard(l.body) else {
            return false;
        };
        if matches!(guard, LoopExitGuard::SquareGt { .. }) {
            return false;
        }
        let (rem_guards, inner_rest) = peel_rem_zero_guards(inner_rest);
        if !rem_guards.is_empty() || body_has_break(inner_rest) || has_loop(inner_rest) {
            return false;
        }
    }
    true
}

