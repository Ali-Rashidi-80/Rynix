fn if_stmt_updates_mut(i: &rynix_ast::IfStmt<'_>) -> bool {
    i.arms.iter().any(|a| if_updates_carried(&a.body))
        || i.else_body.is_some_and(|b| if_updates_carried(b))
}

fn match_stmt_updates_mut(m: &rynix_ast::MatchStmt<'_>) -> bool {
    m.arms.iter().any(|a| if_updates_carried(&a.body))
        || m.else_body.is_some_and(|b| if_updates_carried(b))
}

/// Whether any terminator currently targets `join` (used before sealing empty joins).
fn block_has_incoming(func: &crate::ir::Function, join: BlockId) -> bool {
    for block in &func.blocks {
        let Some(&last) = block.insts.last() else {
            continue;
        };
        match func.inst(last) {
            Inst::Jump { target, .. } if *target == join => return true,
            Inst::Br {
                then_target,
                else_target,
                ..
            } if *then_target == join || *else_target == join => return true,
            _ => {}
        }
    }
    false
}

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
        Expr::StructLit(s) => s
            .fields
            .iter()
            .any(|init| expr_calls_user_fn(init.value, fn_map, _self_name)),
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

