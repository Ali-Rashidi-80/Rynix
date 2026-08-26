//! AST walks for expr / path / ident under a byte offset.

use rynix_ast::{Expr, Item, Module, Path as AstPath, Stmt};
use rynix_ast::Ident;

pub(crate) fn find_expr_at(module: &Module<'_>, offset: u32) -> Option<rynix_ast::NodeId> {
    let mut best: Option<(u32, rynix_ast::NodeId)> = None;
    for item in module.items {
        if let Item::Fn(f) = item {
            for stmt in f.body {
                walk_stmt_for_expr(stmt, offset, &mut best);
            }
        }
    }
    best.map(|(_, id)| id)
}

pub(crate) fn consider_expr(expr: &Expr<'_>, offset: u32, best: &mut Option<(u32, rynix_ast::NodeId)>) {
    if !expr.span().contains(offset) {
        return;
    }
    let len = expr.span().len();
    if best.as_ref().is_none_or(|(l, _)| len <= *l) {
        *best = Some((len, expr.id()));
    }
    match expr {
        Expr::Binary(b) => {
            consider_expr(b.lhs, offset, best);
            consider_expr(b.rhs, offset, best);
        }
        Expr::Unary(u) => consider_expr(u.operand, offset, best),
        Expr::Call(c) => {
            consider_expr(c.callee, offset, best);
            for a in c.args {
                consider_expr(a, offset, best);
            }
        }
        Expr::MethodCall(m) => {
            consider_expr(m.receiver, offset, best);
            for a in m.args {
                consider_expr(a, offset, best);
            }
        }
        Expr::Field(f) => consider_expr(f.base, offset, best),
        Expr::StructLit(s) => {
            for init in s.fields {
                consider_expr(init.value, offset, best);
            }
        }
        Expr::Index(i) => {
            consider_expr(i.base, offset, best);
            consider_expr(i.index, offset, best);
        }
        Expr::Cast(c) => consider_expr(c.expr, offset, best),
        Expr::Array(a) => {
            for e in a.elems {
                consider_expr(e, offset, best);
            }
        }
        Expr::Spawn(s) => consider_expr(s.callee, offset, best),
        Expr::Path(_) | Expr::Literal(_) | Expr::Error(_) => {}
    }
}

pub(crate) fn walk_stmt_for_expr(stmt: &Stmt<'_>, offset: u32, best: &mut Option<(u32, rynix_ast::NodeId)>) {
    match stmt {
        Stmt::Let(l) => consider_expr(l.init, offset, best),
        Stmt::Expr(e) => consider_expr(&e.expr, offset, best),
        Stmt::If(i) => {
            for arm in i.arms {
                consider_expr(arm.cond, offset, best);
                for s in arm.body {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
            if let Some(el) = i.else_body {
                for s in el {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
        }
        Stmt::Match(m) => {
            consider_expr(m.scrutinee, offset, best);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::For(f) => {
            consider_expr(f.iter, offset, best);
            for s in f.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                consider_expr(e, offset, best);
            }
        }
        Stmt::Assign(a) => {
            consider_expr(a.target, offset, best);
            consider_expr(a.value, offset, best);
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
    }
}

pub(crate) fn find_path_at<'a>(module: &Module<'a>, offset: u32) -> Option<&'a AstPath<'a>> {
    let mut found: Option<&AstPath> = None;
    for item in module.items {
        walk_item(item, &mut |path: &AstPath| {
            if path.span.contains(offset) || path_segment_contains(path, offset) {
                found = Some(path);
            }
        });
    }
    found
}

pub(crate) fn path_segment_contains(path: &AstPath, offset: u32) -> bool {
    path.segments
        .iter()
        .any(|s| s.span.contains(offset))
}

pub(crate) fn find_ident_at<'a>(module: &Module<'a>, offset: u32) -> Option<Ident> {
    let mut found = None;
    for item in module.items {
        walk_item_idents(item, offset, &mut found);
    }
    found
}

pub(crate) fn walk_item_idents(item: &Item<'_>, offset: u32, found: &mut Option<Ident>) {
    match item {
        Item::Fn(f) => {
            if f.name.span.contains(offset) {
                *found = Some(f.name);
            }
            for p in f.params {
                if p.name.span.contains(offset) {
                    *found = Some(p.name);
                }
            }
            for stmt in f.body {
                walk_stmt_idents(stmt, offset, found);
            }
        }
        Item::Struct(s) => {
            if s.name.span.contains(offset) {
                *found = Some(s.name);
            }
        }
        Item::Enum(e) => {
            if e.name.span.contains(offset) {
                *found = Some(e.name);
            }
            for v in e.variants {
                if v.name.span.contains(offset) {
                    *found = Some(v.name);
                }
            }
        }
        Item::TypeAlias(t) => {
            if t.name.span.contains(offset) {
                *found = Some(t.name);
            }
        }
        _ => {}
    }
}

pub(crate) fn walk_stmt_idents(stmt: &Stmt<'_>, offset: u32, found: &mut Option<Ident>) {
    match stmt {
        Stmt::Let(l) => {
            if l.name.span.contains(offset) {
                *found = Some(l.name);
            }
            walk_expr_idents(l.init, offset, found);
        }
        Stmt::Expr(e) => walk_expr_idents(&e.expr, offset, found),
        Stmt::If(i) => walk_if_idents(i, offset, found),
        Stmt::Match(m) => {
            walk_expr_idents(m.scrutinee, offset, found);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt_idents(s, offset, found);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt_idents(s, offset, found);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::For(f) => {
            if f.binder.span.contains(offset) {
                *found = Some(f.binder);
            }
            walk_expr_idents(f.iter, offset, found);
            for s in f.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                walk_expr_idents(e, offset, found);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        Stmt::Assign(a) => {
            walk_expr_idents(a.target, offset, found);
            walk_expr_idents(a.value, offset, found);
        }
    }
}

pub(crate) fn walk_if_idents(i: &rynix_ast::IfStmt<'_>, offset: u32, found: &mut Option<Ident>) {
    for arm in i.arms {
        walk_expr_idents(arm.cond, offset, found);
        for s in arm.body {
            walk_stmt_idents(s, offset, found);
        }
    }
    if let Some(el) = i.else_body {
        for s in el {
            walk_stmt_idents(s, offset, found);
        }
    }
}

pub(crate) fn walk_expr_idents(expr: &Expr<'_>, offset: u32, found: &mut Option<Ident>) {
    match expr {
        Expr::Path(p) => {
            for seg in p.segments {
                if seg.span.contains(offset) {
                    *found = Some(*seg);
                }
            }
        }
        Expr::Binary(b) => {
            walk_expr_idents(b.lhs, offset, found);
            walk_expr_idents(b.rhs, offset, found);
        }
        Expr::Unary(u) => walk_expr_idents(u.operand, offset, found),
        Expr::Call(c) => {
            walk_expr_idents(c.callee, offset, found);
            for a in c.args {
                walk_expr_idents(a, offset, found);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr_idents(m.receiver, offset, found);
            for a in m.args {
                walk_expr_idents(a, offset, found);
            }
        }
        Expr::Field(f) => walk_expr_idents(f.base, offset, found),
        Expr::StructLit(s) => {
            for seg in s.path.segments {
                if seg.span.contains(offset) {
                    *found = Some(*seg);
                }
            }
            for init in s.fields {
                if init.name.span.contains(offset) {
                    *found = Some(init.name);
                }
                walk_expr_idents(init.value, offset, found);
            }
        }
        Expr::Index(i) => {
            walk_expr_idents(i.base, offset, found);
            walk_expr_idents(i.index, offset, found);
        }
        Expr::Cast(c) => walk_expr_idents(c.expr, offset, found),
        Expr::Array(a) => {
            for e in a.elems {
                walk_expr_idents(e, offset, found);
            }
        }
        Expr::Spawn(s) => walk_expr_idents(s.callee, offset, found),
        Expr::Literal(_) | Expr::Error(_) => {}
    }
}

pub(crate) fn walk_item<'a>(item: &Item<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match item {
        Item::Fn(f) => {
            for stmt in f.body {
                walk_stmt(stmt, on_path);
            }
        }
        _ => {}
    }
}

pub(crate) fn walk_stmt<'a>(stmt: &Stmt<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match stmt {
        Stmt::Let(l) => walk_expr(l.init, on_path),
        Stmt::Expr(e) => walk_expr(&e.expr, on_path),
        Stmt::If(i) => walk_if(i, on_path),
        Stmt::Match(m) => {
            walk_expr(m.scrutinee, on_path);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt(s, on_path);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt(s, on_path);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::For(f) => {
            walk_expr(f.iter, on_path);
            for s in f.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                walk_expr(e, on_path);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        Stmt::Assign(a) => {
            walk_expr(a.target, on_path);
            walk_expr(a.value, on_path);
        }
    }
}

pub(crate) fn walk_if<'a>(i: &rynix_ast::IfStmt<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    for arm in i.arms {
        walk_expr(arm.cond, on_path);
        for s in arm.body {
            walk_stmt(s, on_path);
        }
    }
    if let Some(el) = i.else_body {
        for s in el {
            walk_stmt(s, on_path);
        }
    }
}

pub(crate) fn walk_expr<'a>(expr: &Expr<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match expr {
        Expr::Path(p) => on_path(p),
        Expr::Binary(b) => {
            walk_expr(b.lhs, on_path);
            walk_expr(b.rhs, on_path);
        }
        Expr::Unary(u) => walk_expr(u.operand, on_path),
        Expr::Call(c) => {
            walk_expr(c.callee, on_path);
            for a in c.args {
                walk_expr(a, on_path);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr(m.receiver, on_path);
            for a in m.args {
                walk_expr(a, on_path);
            }
        }
        Expr::Field(f) => walk_expr(f.base, on_path),
        Expr::StructLit(s) => {
            on_path(s.path);
            for init in s.fields {
                walk_expr(init.value, on_path);
            }
        }
        Expr::Index(i) => {
            walk_expr(i.base, on_path);
            walk_expr(i.index, on_path);
        }
        Expr::Cast(c) => walk_expr(c.expr, on_path),
        Expr::Array(a) => {
            for e in a.elems {
                walk_expr(e, on_path);
            }
        }
        Expr::Spawn(s) => walk_expr(s.callee, on_path),
        Expr::Literal(_) | Expr::Error(_) => {}
    }
}

