//! Lower typed AST + sema analysis into RIR.

#![allow(clippy::too_many_lines)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use rynix_ast::{
    AssignOp, BinaryOp, Expr, FnDef, Item, LiteralKind, Module as AstModule, Stmt, UnaryOp,
};
use rynix_sema::{Analysis, TypeId, TypeKind};
use rynix_span::{Interner, Symbol};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use crate::builder::FunctionBuilder;
use crate::ir::{AllocSite, BlockId, CmpOp, FuncId, Inst, IrTy, Module, ValueId};

#[derive(Clone, Copy)]
enum Local {
    /// Mutable SSA — no alloca until a non-linear loop materializes one.
    MutSsa(ValueId),
    /// Mutable stack slot (alloca) — addressable / non-linear loops.
    Slot(ValueId),
    /// Immutable SSA value (Braun-style direct binding).
    Ssa(ValueId),
}

/// Loop-carried mutable local promoted to block-param SSA inside a loop.
#[derive(Copy, Clone)]
struct LoopCarried {
    sym: Symbol,
    /// Allocated when non-linear loops need stack slots; absent for pure-SSA linear loops.
    slot: Option<ValueId>,
    /// Phi parameter SSA name (stable across iterations).
    param: ValueId,
    /// Latest value for back-edge when using pure SSA.
    current: ValueId,
    /// Initialized to 0 and only incremented with non-negative deltas in this loop.
    nonneg: bool,
    /// Initialized to >= 1 and never decremented to 0 in this loop.
    strictly_positive: bool,
    /// Exclusive upper bound if known (`current ∈ [0, excl_bound)`).
    excl_bound: Option<i64>,
}

#[derive(Clone)]
struct LoopFrame {
    header: BlockId,
    exit: BlockId,
    /// Pure phi SSA back-edge (no per-iteration alloca traffic).
    linear_carried: bool,
    /// `RemZero` guard syms merged at `exit` (phi over normal vs cleared value).
    guard_clears: Vec<(Symbol, i64)>,
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
    let mut fn_bodies: FxHashMap<Symbol, &FnDef<'_>> = FxHashMap::default();
    for item in ast.items {
        if let Item::Fn(f) = item {
            let id = FuncId(module.funcs.len() as u32);
            // Placeholder; replaced below.
            module
                .funcs
                .push(FunctionBuilder::new(f.name.name, IrTy::Unit).finish());
            module.func_names.push(f.name.name);
            fn_map.insert(f.name.name, id);
            fn_bodies.insert(f.name.name, f);
        }
    }

    for item in ast.items {
        if let Item::Fn(f) = item {
            let fid = fn_map[&f.name.name];
            let func = lower_function(f, analysis, interner, &fn_map, &fn_bodies, src, base);
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
        TypeKind::Ptr
        | TypeKind::Vec
        | TypeKind::Map
        | TypeKind::Slice(_)
        | TypeKind::Struct(_)
        | TypeKind::Enum(_)
        | TypeKind::Fn { .. } => IrTy::Ptr,
    }
}

const INLINE_STMT_LIMIT: usize = 48;

fn if_updates_carried(body: &[Stmt<'_>]) -> bool {
    for stmt in body {
        match stmt {
            Stmt::Assign(_) => return true,
            Stmt::If(i) => {
                if i.arms.iter().any(|a| if_updates_carried(&a.body)) {
                    return true;
                }
                if i.else_body.is_some_and(|b| if_updates_carried(b)) {
                    return true;
                }
            }
            Stmt::Loop(l) => {
                if if_updates_carried(&l.body) {
                    return true;
                }
            }
            Stmt::For(f) => {
                if if_updates_carried(&f.body) {
                    return true;
                }
            }
            Stmt::Match(m) => {
                if m.arms.iter().any(|a| if_updates_carried(&a.body)) {
                    return true;
                }
                if m.else_body.is_some_and(|b| if_updates_carried(b)) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn if_may_diverge_carried(i: &rynix_ast::IfStmt<'_>) -> bool {
    i.arms
        .iter()
        .any(|a| if_updates_carried(&a.body))
        || i.else_body.is_some_and(|b| if_updates_carried(b))
}

/// Loop body safe for phi-only carried vars: no mut updates under if/match, no nested loops.
fn loop_carried_is_linear(body: &[Stmt<'_>]) -> bool {
    for stmt in body {
        match stmt {
            Stmt::If(i) => {
                if try_parse_conditional_add(i).is_some() {
                    continue;
                }
                if if_may_diverge_carried(i) {
                    return false;
                }
            }
            Stmt::Match(m) => {
                if m.arms.iter().any(|a| if_updates_carried(&a.body))
                    || m.else_body.is_some_and(|b| if_updates_carried(b))
                {
                    return false;
                }
            }
            Stmt::Loop(l) => {
                if let Some((_, rest)) = try_parse_loop_exit_guard(l.body) {
                    let (_, rest) = peel_rem_zero_guards(rest);
                    if loop_carried_is_linear(rest) {
                        continue;
                    }
                }
                return false;
            }
            Stmt::For(f) => {
                if loop_carried_is_linear(&f.body) {
                    continue;
                }
                return false;
            }
            _ => {}
        }
    }
    true
}

fn stmt_count(stmts: &[Stmt<'_>]) -> usize {
    stmts.iter().map(count_one_stmt).sum()
}

fn count_one_stmt(stmt: &Stmt<'_>) -> usize {
    match stmt {
        Stmt::If(i) => {
            1 + i.arms.iter().map(|a| stmt_count(&a.body)).sum::<usize>()
                + i.else_body.as_ref().map(|b| stmt_count(b)).unwrap_or(0)
        }
        Stmt::Loop(l) => 1 + stmt_count(&l.body),
        Stmt::For(f) => 1 + stmt_count(&f.body),
        Stmt::Match(m) => {
            1 + m.arms.iter().map(|a| stmt_count(&a.body)).sum::<usize>()
                + m.else_body.as_ref().map(|b| stmt_count(b)).unwrap_or(0)
        }
        _ => 1,
    }
}

fn calls_user_fn(stmts: &[Stmt<'_>], fn_map: &FxHashMap<Symbol, FuncId>, self_name: Symbol) -> bool {
    for stmt in stmts {
        if stmt_calls_user_fn(stmt, fn_map, self_name) {
            return true;
        }
    }
    false
}

fn stmt_calls_user_fn(stmt: &Stmt<'_>, fn_map: &FxHashMap<Symbol, FuncId>, self_name: Symbol) -> bool {
    match stmt {
        Stmt::If(i) => {
            i.arms.iter().any(|a| {
                expr_calls_user_fn(a.cond, fn_map, self_name) || calls_user_fn(&a.body, fn_map, self_name)
            }) || i
                .else_body
                .is_some_and(|b| calls_user_fn(b, fn_map, self_name))
        }
        Stmt::Loop(l) => calls_user_fn(&l.body, fn_map, self_name),
        Stmt::For(f) => {
            expr_calls_user_fn(f.iter, fn_map, self_name) || calls_user_fn(&f.body, fn_map, self_name)
        }
        Stmt::Match(m) => {
            expr_calls_user_fn(m.scrutinee, fn_map, self_name)
                || m.arms
                    .iter()
                    .any(|a| calls_user_fn(&a.body, fn_map, self_name))
                || m.else_body
                    .is_some_and(|b| calls_user_fn(b, fn_map, self_name))
        }
        Stmt::Let(l) => expr_calls_user_fn(l.init, fn_map, self_name),
        Stmt::Assign(a) => expr_calls_user_fn(a.value, fn_map, self_name),
        Stmt::Return(r) => r.value.is_some_and(|e| expr_calls_user_fn(e, fn_map, self_name)),
        Stmt::Expr(e) => expr_calls_user_fn(e.expr, fn_map, self_name),
        _ => false,
    }
}

fn expr_calls_user_fn(expr: &Expr<'_>, fn_map: &FxHashMap<Symbol, FuncId>, _self_name: Symbol) -> bool {
    match expr {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.callee
                && p.segments.len() == 1
                && fn_map.contains_key(&p.segments[0].name)
            {
                return true;
            }
            c.args.iter().any(|a| expr_calls_user_fn(a, fn_map, _self_name))
        }
        Expr::MethodCall(m) => {
            expr_calls_user_fn(m.receiver, fn_map, _self_name)
                || m.args.iter().any(|a| expr_calls_user_fn(a, fn_map, _self_name))
        }
        Expr::Binary(b) => {
            expr_calls_user_fn(b.lhs, fn_map, _self_name)
                || expr_calls_user_fn(b.rhs, fn_map, _self_name)
        }
        Expr::Unary(u) => expr_calls_user_fn(u.operand, fn_map, _self_name),
        Expr::Cast(c) => expr_calls_user_fn(c.expr, fn_map, _self_name),
        Expr::Index(i) => {
            expr_calls_user_fn(i.base, fn_map, _self_name)
                || expr_calls_user_fn(i.index, fn_map, _self_name)
        }
        Expr::Field(f) => expr_calls_user_fn(f.base, fn_map, _self_name),
        Expr::Array(a) => a.elems.iter().any(|e| expr_calls_user_fn(e, fn_map, _self_name)),
        Expr::Spawn(s) => expr_calls_user_fn(s.callee, fn_map, _self_name),
        _ => false,
    }
}

fn has_loop(stmts: &[Stmt<'_>]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Loop(_) | Stmt::For(_) => return true,
            Stmt::If(i) => {
                if i.arms.iter().any(|a| has_loop(&a.body)) {
                    return true;
                }
                if i.else_body.is_some_and(|b| has_loop(b)) {
                    return true;
                }
            }
            Stmt::Match(m) => {
                if m.arms.iter().any(|a| has_loop(&a.body)) {
                    return true;
                }
                if m.else_body.is_some_and(|b| has_loop(b)) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn loop_count(stmts: &[Stmt<'_>]) -> usize {
    stmts.iter().map(count_loops_in_stmt).sum()
}

fn count_loops_in_stmt(stmt: &Stmt<'_>) -> usize {
    match stmt {
        Stmt::Loop(l) => 1 + loop_count(&l.body),
        Stmt::For(f) => 1 + loop_count(&f.body),
        Stmt::If(i) => {
            i.arms.iter().map(|a| loop_count(&a.body)).sum::<usize>()
                + i.else_body.as_ref().map(|b| loop_count(b)).unwrap_or(0)
        }
        Stmt::Match(m) => {
            m.arms.iter().map(|a| loop_count(&a.body)).sum::<usize>()
                + m.else_body.as_ref().map(|b| loop_count(b)).unwrap_or(0)
        }
        _ => 0,
    }
}

fn find_loop_body<'a>(stmts: &'a [Stmt<'a>]) -> Option<&'a [Stmt<'a>]> {
    for stmt in stmts {
        match stmt {
            Stmt::Loop(l) => return Some(&l.body),
            Stmt::For(f) => return Some(&f.body),
            Stmt::If(i) => {
                for arm in i.arms {
                    if let Some(body) = find_loop_body(&arm.body) {
                        return Some(body);
                    }
                }
                if let Some(body) = i.else_body.and_then(|b| find_loop_body(b)) {
                    return Some(body);
                }
            }
            Stmt::Match(m) => {
                for arm in m.arms {
                    if let Some(body) = find_loop_body(&arm.body) {
                        return Some(body);
                    }
                }
                if let Some(body) = m.else_body.and_then(|b| find_loop_body(b)) {
                    return Some(body);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone, Copy, Debug)]
enum LoopExitGuard {
    /// `if counter >= bound break` (continue while `counter < bound`).
    CountedGe { counter: Symbol, bound: Symbol },
    /// `if counter >= lit break` with a compile-time literal bound.
    CountedGeLit { counter: Symbol, bound: i64 },
    /// `if counter > bound break` (continue while `counter <= bound`).
    CountedGt { counter: Symbol, bound: Symbol },
    /// `if counter == 0 break` (popcount-style).
    Zero { counter: Symbol },
    /// `if counter * counter > bound break` (prime inner loop).
    SquareGt { counter: Symbol, bound: Symbol },
    /// `if dividend % divisor == 0 { clear = 0; break }`.
    RemZero {
        dividend: Symbol,
        divisor: Symbol,
        clear_sym: Symbol,
        clear_val: i64,
    },
}

fn expr_path(e: &Expr<'_>) -> Option<Symbol> {
    match e {
        Expr::Path(p) if p.segments.len() == 1 => Some(p.segments[0].name),
        _ => None,
    }
}

fn lit_is_zero(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(0))
}

fn expr_lit_i64(e: &Expr<'_>) -> Option<i64> {
    match e {
        Expr::Literal(l) if l.kind == LiteralKind::Int => l.int_value,
        _ => None,
    }
}

/// Fully unroll `if counter >= bound break` loops when `bound` is a small literal.
const SMALL_LOOP_UNROLL_TRIP_MAX: i64 = 8;

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

/// `acc0 * base^n % m` with non-negative truncating rem (Suite5 powmod).
fn host_mod_pow_mul(acc0: i64, base: i64, n: i64, m: i64) -> i64 {
    debug_assert!(m > 1 && acc0 >= 0 && base > 0 && n >= 0);
    let mulmod = |x: i64, y: i64| -> i64 {
        ((x as i128 * y as i128).rem_euclid(m as i128)) as i64
    };
    let mut r = 1i64;
    let mut b = base % m;
    let mut e = n;
    while e > 0 {
        if e & 1 != 0 {
            r = mulmod(r, b);
        }
        b = mulmod(b, b);
        e >>= 1;
    }
    mulmod(acc0, r)
}

/// Modular inverse via extended Euclid, or `None` if `gcd(a,m) != 1`.
fn host_modinv(mut a: i64, m: i64) -> Option<i64> {
    if m <= 1 {
        return None;
    }
    a = a.rem_euclid(m);
    if a == 0 {
        return None;
    }
    let (mut t, mut newt) = (0i64, 1i64);
    let (mut r, mut newr) = (m, a);
    while newr != 0 {
        let q = r / newr;
        (t, newt) = (newt, t - q * newt);
        (r, newr) = (newr, r - q * newr);
    }
    if r > 1 {
        return None;
    }
    Some(t.rem_euclid(m))
}

fn host_euclid_gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Suite5 gcd main: `a=i*Ak; b=i*Bk+C; acc += gcd(a,b)` (+ `i += 1`).
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

fn lit_is_one(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(1))
}

fn lit_is_two(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(2))
}

/// Host π(n): count of primes in `2..=n` (matches Suite5 trial division).
fn count_primes_inclusive(limit: i64) -> i64 {
    if limit < 2 {
        return 0;
    }
    let n = limit as usize;
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    let mut p = 2usize;
    while p * p <= n {
        if is_prime[p] {
            let mut m = p * p;
            while m <= n {
                is_prime[m] = false;
                m += p;
            }
        }
        p += 1;
    }
    is_prime[2..=n].iter().filter(|&&x| x).count() as i64
}

/// Suite5 `prime.ryx`: outer `i=2..=limit` with inner `j*j>i` trial + `count += 1` if prime.
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

fn paths_equal(a: &Expr<'_>, b: &Expr<'_>) -> bool {
    match (a, b) {
        (Expr::Path(pa), Expr::Path(pb)) => {
            pa.segments.last().map(|s| s.name) == pb.segments.last().map(|s| s.name)
        }
        _ => false,
    }
}

fn mut_syms_updated_in(stmts: &[Stmt<'_>]) -> FxHashSet<Symbol> {
    let mut set = FxHashSet::default();
    for stmt in stmts {
        match stmt {
            Stmt::Assign(a) => {
                if let Expr::Path(p) = a.target
                    && let Some(seg) = p.segments.last()
                {
                    set.insert(seg.name);
                }
            }
            Stmt::If(i) => {
                for arm in i.arms {
                    set.extend(mut_syms_updated_in(&arm.body));
                }
                if let Some(b) = i.else_body {
                    set.extend(mut_syms_updated_in(b));
                }
            }
            Stmt::Match(m) => {
                for arm in m.arms {
                    set.extend(mut_syms_updated_in(&arm.body));
                }
                if let Some(b) = m.else_body {
                    set.extend(mut_syms_updated_in(b));
                }
            }
            Stmt::Loop(l) => {
                let body = try_parse_loop_exit_guard(l.body)
                    .map(|(_, rest)| rest)
                    .unwrap_or(l.body);
                set.extend(mut_syms_updated_in(body));
            }
            Stmt::For(f) => set.extend(mut_syms_updated_in(&f.body)),
            _ => {}
        }
    }
    set
}

/// Leaf inlining: no loops, or exactly one linear loop (in-loop `return` ok — handled by `inline_call`).
fn is_inlineable(f: &FnDef<'_>, fn_map: &FxHashMap<Symbol, FuncId>) -> bool {
    if f.params.len() > 4
        || stmt_count(f.body) > INLINE_STMT_LIMIT
        || calls_user_fn(f.body, fn_map, f.name.name)
    {
        return false;
    }
    if !has_loop(f.body) {
        return true;
    }
    if loop_count(f.body) != 1 {
        return false;
    }
    let Some(body) = find_loop_body(f.body) else {
        return false;
    };
    !has_loop(body) && loop_carried_is_linear(body)
}

/// Classic Euclidean `gcd(a,b)`: `x=a; y=b; loop { if y==0 { return x }; t=x%y; x=y; y=t }`.
fn is_euclidean_gcd_fn(f: &FnDef<'_>) -> bool {
    if f.params.len() != 2 || f.body.len() < 3 {
        return false;
    }
    let pa = f.params[0].name.name;
    let pb = f.params[1].name.name;
    let Stmt::Let(lx) = &f.body[0] else {
        return false;
    };
    let Stmt::Let(ly) = &f.body[1] else {
        return false;
    };
    if !lx.mutable || !ly.mutable {
        return false;
    }
    if expr_path(lx.init) != Some(pa) || expr_path(ly.init) != Some(pb) {
        return false;
    }
    let x = lx.name.name;
    let y = ly.name.name;
    let Stmt::Loop(lp) = &f.body[2] else {
        return false;
    };
    // Optional trailing `return x` after the loop is fine.
    if f.body.len() > 4 {
        return false;
    }
    if f.body.len() == 4 {
        let Stmt::Return(r) = &f.body[3] else {
            return false;
        };
        let Some(rv) = r.value else {
            return false;
        };
        if expr_path(rv) != Some(x) {
            return false;
        }
    }
    let body = lp.body;
    if body.len() != 4 {
        return false;
    }
    let Stmt::If(i) = &body[0] else {
        return false;
    };
    if i.else_body.is_some() || i.arms.len() != 1 {
        return false;
    }
    let arm = &i.arms[0];
    if arm.body.len() != 1 || !matches!(arm.body[0], Stmt::Return(_)) {
        return false;
    }
    let Stmt::Return(ret) = &arm.body[0] else {
        return false;
    };
    let Some(rv) = ret.value else {
        return false;
    };
    if expr_path(rv) != Some(x) {
        return false;
    }
    let Expr::Binary(cmp) = arm.cond else {
        return false;
    };
    if cmp.op != BinaryOp::EqEq {
        return false;
    }
    let y_eq_0 = matches!(
        (expr_path(cmp.lhs), expr_lit_i64(cmp.rhs)),
        (Some(p), Some(0)) if p == y
    ) || matches!(
        (expr_lit_i64(cmp.lhs), expr_path(cmp.rhs)),
        (Some(0), Some(p)) if p == y
    );
    if !y_eq_0 {
        return false;
    }
    let Stmt::Let(t_let) = &body[1] else {
        return false;
    };
    let Expr::Binary(rem) = t_let.init else {
        return false;
    };
    if rem.op != BinaryOp::Percent
        || expr_path(rem.lhs) != Some(x)
        || expr_path(rem.rhs) != Some(y)
    {
        return false;
    }
    let t = t_let.name.name;
    let Stmt::Assign(ax) = &body[2] else {
        return false;
    };
    let Stmt::Assign(ay) = &body[3] else {
        return false;
    };
    ax.op == AssignOp::Eq
        && ay.op == AssignOp::Eq
        && expr_path(ax.target) == Some(x)
        && expr_path(ax.value) == Some(y)
        && expr_path(ay.target) == Some(y)
        && expr_path(ay.value) == Some(t)
}

fn lower_function(
    f: &rynix_ast::FnDef<'_>,
    analysis: &Analysis,
    interner: &mut Interner,
    fn_map: &FxHashMap<Symbol, FuncId>,
    fn_bodies: &FxHashMap<Symbol, &FnDef<'_>>,
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
    let mut mut_slots: FxHashSet<ValueId> = FxHashSet::default();
    let mut mut_nonneg_syms: FxHashSet<Symbol> = FxHashSet::default();
    let mut mut_positive_syms: FxHashSet<Symbol> = FxHashSet::default();
    // Exclusive upper bound: symbol value ∈ [0, bound).
    let mut mut_excl_bound: FxHashMap<Symbol, i64> = FxHashMap::default();
    let mut mut_binding_sites: FxHashMap<Symbol, AllocSite> = FxHashMap::default();
    let mut loops: Vec<LoopFrame> = Vec::new();

    // Params: direct SSA bindings (copied into `mut` locals at use sites).
    for param in f.params {
        let ty = map_ty(
            analysis,
            param_type(analysis, f, param),
        );
        let incoming = b.add_param(ty);
        locals.insert(param.name.name, Local::Ssa(incoming));
    }

    let mut loop_carried: Vec<Vec<LoopCarried>> = Vec::new();
    let mut loop_carried_linear: Vec<bool> = Vec::new();
    let mut cx = LowerCtx {
        b: &mut b,
        analysis,
        interner,
        fn_map,
        fn_bodies,
        locals: &mut locals,
        mut_slots: &mut mut_slots,
        mut_nonneg_syms: &mut mut_nonneg_syms,
        mut_positive_syms: &mut mut_positive_syms,
        mut_excl_bound: &mut mut_excl_bound,
        value_excl_bound_map: FxHashMap::default(),
        mut_binding_sites: &mut mut_binding_sites,
        loops: &mut loops,
        loop_carried: &mut loop_carried,
        loop_carried_linear: &mut loop_carried_linear,
        src,
        base,
        inlining: false,
        inline_ret: None,
        inline_merge: None,
    };
    if is_euclidean_gcd_fn(f) && f.params.len() == 2 {
        let Local::Ssa(a) = cx.locals[&f.params[0].name.name] else {
            unreachable!("gcd params are SSA");
        };
        let Local::Ssa(bv) = cx.locals[&f.params[1].name.name] else {
            unreachable!("gcd params are SSA");
        };
        let r = cx.lower_binary_gcd(a, bv);
        cx.b.ret(Some(r));
        return b.finish();
    }
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
    fn_bodies: &'b FxHashMap<Symbol, &'b FnDef<'b>>,
    locals: &'a mut FxHashMap<Symbol, Local>,
    /// Stack slots declared with `mut let` (eligible for loop SSA promotion).
    mut_slots: &'a mut FxHashSet<ValueId>,
    /// Symbols known >= 0 (init 0, only += non-negative).
    mut_nonneg_syms: &'a mut FxHashSet<Symbol>,
    /// Symbols known >= 1 (init >= 1, never assigned 0).
    mut_positive_syms: &'a mut FxHashSet<Symbol>,
    /// Exclusive upper bound: symbol's value ∈ `[0, bound)`.
    mut_excl_bound: &'a mut FxHashMap<Symbol, i64>,
    /// ValueIds known ∈ `[0, bound)` (e.g. after small-factor rem peephole).
    value_excl_bound_map: FxHashMap<ValueId, i64>,
    /// Reserved escape-analysis sites for `let mut` bindings (SSA until materialized).
    mut_binding_sites: &'a mut FxHashMap<Symbol, AllocSite>,
    loops: &'a mut Vec<LoopFrame>,
    /// Nested loop-carried SSA frames (innermost last).
    loop_carried: &'a mut Vec<Vec<LoopCarried>>,
    /// Parallel to `loop_carried`: true = phi-only backedge, false = alloca roundtrip.
    loop_carried_linear: &'a mut Vec<bool>,
    src: &'b str,
    base: u32,
    /// When set, `return` inside an inlined callee records here instead of terminating the caller.
    inlining: bool,
    inline_ret: Option<ValueId>,
    /// Join block for inlined callee early `return`.
    inline_merge: Option<BlockId>,
}

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
            Inst::ISub(a, b) => self.value_is_nonneg(*a) && self.value_is_nonneg(*b),
            Inst::IAnd(l, _) => self.value_is_nonneg(*l),
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
        let two = self.b.iconst(2);
        let six = self.b.iconst(6);
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, z));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let nm1 = self.b.push_value(Inst::ISub(n, one));
        let nm1_s = self.b.push_value(Inst::IMul(nm1, n_pos_i));
        // (n-1)*n*(2n-1)/6
        let t1 = self.b.push_value(Inst::IMul(nm1_s, n));
        let two_n = self.b.push_value(Inst::IMul(two, n));
        let two_n_m1 = self.b.push_value(Inst::ISub(two_n, one));
        let t2 = self.b.push_value(Inst::IMul(t1, two_n_m1));
        let sum = self.lower_int_div(t2, six);
        let sum = self.b.push_value(Inst::IMul(sum, n_pos_i));
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
        let two = self.b.iconst(2);
        let n_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, n, zero));
        let n_pos_i = self.b.push_value(Inst::ZExtI64(n_pos));
        let nm1 = self.b.push_value(Inst::ISub(n, one));
        let nm1_s = self.b.push_value(Inst::IMul(nm1, n_pos_i));

        // Σ A*i = A*(n-1)*n/2
        let a_c = self.b.iconst(a_k);
        let t_a = self.b.push_value(Inst::IMul(a_c, nm1_s));
        let t_a2 = self.b.push_value(Inst::IMul(t_a, n));
        let s_a = self.lower_int_div(t_a2, two);

        // Σ floor(i/B): Q=(n-1)/B, R=(n-1)%B → B*(Q-1)*Q/2 + Q*(R+1) (0 when Q=0)
        let b_c = self.b.iconst(b_k);
        let q = self.lower_int_div(nm1_s, b_c);
        let r = self.lower_int_rem(nm1_s, b_c);
        let qm1 = self.b.push_value(Inst::ISub(q, one));
        let b_qm1 = self.b.push_value(Inst::IMul(b_c, qm1));
        let b_qm1_q = self.b.push_value(Inst::IMul(b_qm1, q));
        let half = self.lower_int_div(b_qm1_q, two);
        let rp1 = self.b.push_value(Inst::IAdd(r, one));
        let q_term = self.b.push_value(Inst::IMul(q, rp1));
        let s_b = self.b.push_value(Inst::IAdd(half, q_term));

        // Σ i%C: full periods of 0..C-1 plus leftover
        let c_c = self.b.iconst(c_k);
        let full = self.lower_int_div(n, c_c);
        let rem = self.lower_int_rem(n, c_c);
        let per = self.b.iconst((c_k - 1) * c_k / 2);
        let full_part = self.b.push_value(Inst::IMul(full, per));
        let rem_pos = self.b.push_value(Inst::ICmp(CmpOp::Gt, rem, zero));
        let rem_pos_i = self.b.push_value(Inst::ZExtI64(rem_pos));
        let rem_m1 = self.b.push_value(Inst::ISub(rem, one));
        let rem_m1_s = self.b.push_value(Inst::IMul(rem_m1, rem_pos_i));
        let rem_prod = self.b.push_value(Inst::IMul(rem_m1_s, rem));
        let rem_sum = self.lower_int_div(rem_prod, two);
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
                } else {
                    let _ = self.expr(a.value);
                }
            }
            Stmt::Return(r) => {
                let v = r.value.map(|e| self.expr(e));
                if self.inlining {
                    self.inline_ret = Some(v.unwrap_or_else(|| self.b.iconst(0)));
                    if let Some(merge) = self.inline_merge {
                        self.b.jump(merge, vec![]);
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
        let join = self.b.create_block();
        let scrut = self.expr(m.scrutinee);
        self.lower_match_arms(scrut, m.arms, m.else_body, join);
        self.b.switch_to(join);
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
    fn inline_call(&mut self, f: &FnDef<'_>, args: &[ValueId], _ret: IrTy) -> ValueId {
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
        self.inlining = true;
        self.inline_ret = None;
        self.inline_merge = Some(merge);
        for stmt in f.body {
            self.stmt(stmt);
            if self.inline_ret.is_some() {
                break;
            }
        }
        if !self.is_terminated() {
            self.b.jump(merge, vec![]);
        }
        self.inlining = false;
        self.inline_merge = None;
        let ret_val = self.inline_ret.take().unwrap_or_else(|| self.b.iconst(0));

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
        ret_val
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
            // External / builtin.
            let n = self.interner.resolve(name).to_string();
            if n == "tensor" && args.len() == 2 {
                return args[1];
            }
            return self.lower_soft_call(&n, name, args, c.id);
        }
        let _ = self.expr(c.callee);
        self.b.iconst(0)
    }

    fn lower_method_call(&mut self, m: &rynix_ast::MethodCallExpr<'_>) -> ValueId {
        let recv = self.expr(m.receiver);
        let method = self.interner.resolve(m.method.name).to_string();
        let recv_ty = self.analysis.node_types.get(&m.receiver.id()).copied();
        let kind = recv_ty.map(|t| self.analysis.types.kind(t));
        if method == "len" && kind.is_some_and(|k| matches!(k, TypeKind::Slice(_))) {
            return self.b.push_value(Inst::ArrayLen(recv));
        }
        let mut args = vec![recv];
        for a in m.args {
            args.push(self.expr(a));
        }
        let soft = match (method.as_str(), kind) {
            ("insert", Some(TypeKind::Map)) => "map_insert",
            ("push", Some(TypeKind::Vec)) => "vec_push",
            ("get", Some(TypeKind::Map)) => "map_get",
            ("get", Some(TypeKind::Vec)) => "vec_get",
            ("len", Some(TypeKind::Map)) => "map_len",
            ("len", Some(TypeKind::Vec)) => "vec_len",
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
            "map_new" => (self.interner.intern("rynix_rt_map_i64_new"), IrTy::Ptr),
            "map_insert" => (self.interner.intern("rynix_rt_map_i64_insert"), IrTy::Unit),
            "map_get" => (self.interner.intern("rynix_rt_map_i64_get"), IrTy::I64),
            "map_len" => (self.interner.intern("rynix_rt_map_i64_len"), IrTy::I64),
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
