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

/// Guarded-loop peel is safe when nested `loop`s are simple counted exits only
/// (no `j*j > i`, rem-zero peel, or extra `break` in the inner body — see `prime.ryx`).
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

    /// `n = 2^shift - 1` → `(shift, false)`; `n = 2^shift + 1` → `(shift, true)`.
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
        let minus_one = *n + 1;
        if (minus_one & (minus_one - 1)) == 0 {
            return Some((minus_one.trailing_zeros(), false));
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
                {
                    self.mut_nonneg_syms.insert(sym);
                } else {
                    self.mut_nonneg_syms.remove(&sym);
                }
            }
            AssignOp::PlusEq => {
                if !self.value_is_nonneg_iconst(rhs) {
                    self.mut_nonneg_syms.remove(&sym);
                }
            }
            AssignOp::MinusEq
            | AssignOp::StarEq
            | AssignOp::SlashEq
            | AssignOp::PercentEq => {
                self.mut_nonneg_syms.remove(&sym);
            }
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
        let c = self.expr(cond);
        let inc = self.b.push_value(Inst::ZExtI64(c));
        let cur = self.load_sym(target);
        let next = self.b.push_value(Inst::IAdd(cur, inc));
        self.store_sym(target, next);
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
                if let Some((guard, rest)) = try_parse_loop_exit_guard(l.body)
                    && self.loop_guard_eligible(guard)
                    && rest_allows_guarded_loop(rest)
                {
                    if let LoopExitGuard::CountedGeLit { counter, bound } = guard
                        && self.lower_unrolled_counted_ge_loop(counter, bound, rest)
                    {
                        return;
                    }
                    self.lower_guarded_loop(l.span, guard, rest);
                    return;
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
                if bin.op == BinaryOp::Plus {
                    if let Some(v) = self.try_lower_i_mul_plus_i(bin.lhs, bin.rhs) {
                        return v;
                    }
                }
                let l = self.expr(bin.lhs);
                let r = self.expr(bin.rhs);
                if bin.op == BinaryOp::Percent {
                    if let (Some(lsym), Some(rsym)) = (expr_path(bin.lhs), expr_path(bin.rhs))
                        && self.mut_nonneg_syms.contains(&lsym)
                        && self.mut_positive_syms.contains(&rsym)
                    {
                        return self.b.push_value(Inst::URem(l, r));
                    }
                }
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
        }
    }

    fn cmp(&mut self, op: CmpOp, l: ValueId, r: ValueId, floaty: bool) -> ValueId {
        if floaty {
            self.b.push_value(Inst::FCmp(op, l, r))
        } else {
            self.b.push_value(Inst::ICmp(op, l, r))
        }
    }

    /// Expand a small leaf callee in-place (early `return` joins at `inline_merge`).
    fn inline_call(&mut self, f: &FnDef<'_>, args: &[ValueId], _ret: IrTy) -> ValueId {
        let snapshot = self.locals.clone();
        let nonneg_snapshot = self.mut_nonneg_syms.clone();
        let positive_snapshot = self.mut_positive_syms.clone();
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
            "http_get_json_i64" => (self.interner.intern("rynix_rt_http_get_json_i64"), IrTy::I64),
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
