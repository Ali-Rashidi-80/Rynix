//! Static effect sets (`#^ effect: pure`) — Suite A3.
//!
//! Declared purity is checked against inferred effects from soft builtins and
//! intra-module calls (End-style transitive purity; Rynix `#^` syntax).

use rustc_hash::{FxHashMap, FxHashSet};
use rynix_ast::{Expr, Item, Module, Stmt};
use rynix_diag::DiagSink;
use rynix_span::{Interner, Span, Symbol};

use crate::def::{DefId, DefKind};
use crate::errors;

/// Bitset of impure effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EffectSet(u8);

impl EffectSet {
    pub const IO: Self = Self(0b001);
    pub const NETWORK: Self = Self(0b010);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn is_impure(self) -> bool {
        !self.is_empty()
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.contains(Self::IO) {
            parts.push("io");
        }
        if self.contains(Self::NETWORK) {
            parts.push("network");
        }
        if parts.is_empty() {
            "pure".into()
        } else {
            parts.join("+")
        }
    }
}

impl std::ops::BitOrAssign for EffectSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Soft-builtin → effects (conservative; unknown callees stay empty).
pub fn builtin_effects(name: &str) -> EffectSet {
    match name {
        "print" | "print_i64" | "sleep_ms" | "yield" | "now_ms" | "fiber_run"
        | "kv_new" | "kv_put" | "kv_get" | "kv_len" => EffectSet::IO,
        n if n.starts_with("http_") || n.starts_with("tcp_") || n.starts_with("frame_") => {
            EffectSet::IO.union(EffectSet::NETWORK)
        }
        _ => EffectSet::empty(),
    }
}

fn source_line_containing(source: &str, span: Span, base: u32) -> Option<&str> {
    let lo = span.lo().saturating_sub(base) as usize;
    if lo > source.len() {
        return None;
    }
    let start = source[..lo].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = source[lo..]
        .find('\n')
        .map(|i| lo + i)
        .unwrap_or(source.len());
    Some(&source[start..end])
}

/// Parse `#^ effect: pure` (or `#^ effects: pure`) on the source line of `span`.
/// `base` is the file's start offset in the global span space (0 for unit tests).
pub fn line_declares_pure(source: &str, span: Span, base: u32) -> bool {
    let Some(line) = source_line_containing(source, span, base) else {
        return false;
    };
    let Some(idx) = line.find("#^") else {
        return false;
    };
    let rest = line[idx + 2..].trim_start();
    let rest = rest
        .strip_prefix("effects:")
        .or_else(|| rest.strip_prefix("effect:"))
        .unwrap_or("")
        .trim_start();
    rest.split(|c: char| c == ',' || c.is_whitespace())
        .any(|t| t == "pure")
}

/// Infer + check purity for all functions in `module`.
pub fn check_module_effects(
    module: &Module<'_>,
    source: &str,
    source_base: u32,
    interner: &Interner,
    defs: &[DefKind],
    path_resolution: &FxHashMap<rynix_ast::NodeId, DefId>,
    fn_sigs: &FxHashMap<DefId, crate::ty::TypeId>,
    sink: &mut dyn DiagSink,
) -> FxHashMap<DefId, EffectSet> {
    let mut fn_items: Vec<(DefId, Symbol, Span, &rynix_ast::FnDef<'_>)> = Vec::new();
    for item in module.items {
        let Item::Fn(f) = item else {
            continue;
        };
        let def = defs.iter().enumerate().find_map(|(i, k)| match k {
            DefKind::Fn { name, span, .. } if *name == f.name.name && *span == f.name.span => {
                Some(DefId::from_index(i as u32))
            }
            DefKind::Fn { name, .. } if *name == f.name.name => Some(DefId::from_index(i as u32)),
            _ => None,
        });
        let Some(def) = def else {
            continue;
        };
        fn_items.push((def, f.name.name, f.name.span, f));
    }

    let mut direct: FxHashMap<DefId, EffectSet> = FxHashMap::default();
    let mut calls: FxHashMap<DefId, FxHashSet<DefId>> = FxHashMap::default();

    for &(def, _, _, f) in &fn_items {
        let mut set = EffectSet::empty();
        let mut callees = FxHashSet::default();
        walk_stmts(f.body, &mut |expr| {
            collect_call_effects(
                expr,
                interner,
                path_resolution,
                fn_sigs,
                &mut set,
                &mut callees,
            );
        });
        direct.insert(def, set);
        calls.insert(def, callees);
    }

    let mut inferred = direct;
    let mut changed = true;
    while changed {
        changed = false;
        for &(def, _, _, _) in &fn_items {
            let mut next = inferred.get(&def).copied().unwrap_or_else(EffectSet::empty);
            if let Some(callees) = calls.get(&def) {
                for &c in callees {
                    if let Some(ce) = inferred.get(&c) {
                        let u = next.union(*ce);
                        if u != next {
                            next = u;
                            changed = true;
                        }
                    }
                }
            }
            inferred.insert(def, next);
        }
    }

    for &(def, name, span, _) in &fn_items {
        if !line_declares_pure(source, span, source_base) {
            continue;
        }
        let eff = inferred.get(&def).copied().unwrap_or_else(EffectSet::empty);
        if eff.is_impure() {
            sink.emit(errors::purity_violation(
                span,
                interner.resolve(name),
                &eff.label(),
            ));
        }
    }

    inferred
}

fn walk_stmts(stmts: &[Stmt<'_>], f: &mut dyn FnMut(&Expr<'_>)) {
    for s in stmts {
        walk_stmt(s, f);
    }
}

fn walk_stmt(stmt: &Stmt<'_>, f: &mut dyn FnMut(&Expr<'_>)) {
    match stmt {
        Stmt::Let(l) => walk_expr(l.init, f),
        Stmt::Assign(a) => {
            walk_expr(a.target, f);
            walk_expr(a.value, f);
        }
        Stmt::Return(r) => {
            if let Some(v) = r.value {
                walk_expr(v, f);
            }
        }
        Stmt::Expr(e) => walk_expr(e.expr, f),
        Stmt::Loop(l) => walk_stmts(l.body, f),
        Stmt::Region(r) => walk_stmts(r.body, f),
        Stmt::For(fr) => {
            walk_expr(fr.iter, f);
            walk_stmts(fr.body, f);
        }
        Stmt::If(i) => {
            for arm in i.arms {
                walk_expr(arm.cond, f);
                walk_stmts(arm.body, f);
            }
            if let Some(e) = i.else_body {
                walk_stmts(e, f);
            }
        }
        Stmt::Match(m) => {
            walk_expr(m.scrutinee, f);
            for arm in m.arms {
                walk_stmts(arm.body, f);
            }
            if let Some(e) = m.else_body {
                walk_stmts(e, f);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
    }
}

fn walk_expr(expr: &Expr<'_>, f: &mut dyn FnMut(&Expr<'_>)) {
    f(expr);
    match expr {
        Expr::Unary(u) => walk_expr(u.operand, f),
        Expr::Binary(b) => {
            walk_expr(b.lhs, f);
            walk_expr(b.rhs, f);
        }
        Expr::Call(c) => {
            walk_expr(c.callee, f);
            for a in c.args {
                walk_expr(a, f);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr(m.receiver, f);
            for a in m.args {
                walk_expr(a, f);
            }
        }
        Expr::Index(i) => {
            walk_expr(i.base, f);
            walk_expr(i.index, f);
        }
        Expr::Field(fl) => walk_expr(fl.base, f),
        Expr::Cast(c) => walk_expr(c.expr, f),
        Expr::Array(a) => {
            for e in a.elems {
                walk_expr(e, f);
            }
        }
        Expr::Spawn(s) => walk_expr(s.callee, f),
        Expr::Literal(_) | Expr::Path(_) | Expr::Error(_) => {}
    }
}

fn collect_call_effects(
    expr: &Expr<'_>,
    interner: &Interner,
    path_resolution: &FxHashMap<rynix_ast::NodeId, DefId>,
    fn_sigs: &FxHashMap<DefId, crate::ty::TypeId>,
    set: &mut EffectSet,
    callees: &mut FxHashSet<DefId>,
) {
    match expr {
        Expr::Call(c) => {
            if let Expr::Path(p) = c.callee {
                if let Some(&def) = path_resolution.get(&p.id) {
                    if fn_sigs.contains_key(&def) {
                        callees.insert(def);
                    }
                    if let Some(name) = p.segments.last() {
                        *set |= builtin_effects(interner.resolve(name.name));
                    }
                } else if let Some(seg) = p.segments.last() {
                    *set |= builtin_effects(interner.resolve(seg.name));
                }
            }
        }
        Expr::MethodCall(m) => {
            let method = interner.resolve(m.method.name);
            if matches!(method, "push" | "insert") {
                *set |= EffectSet::IO;
            }
        }
        Expr::Spawn(_) => {
            *set |= EffectSet::IO;
        }
        _ => {}
    }
}
