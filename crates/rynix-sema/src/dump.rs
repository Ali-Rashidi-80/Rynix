//! Human-readable type dump for tests and debugging.

use std::fmt::Write as _;

use rynix_ast::{Item, Module, Stmt};
use rynix_span::Interner;

use crate::check::Analysis;

/// Dumps inferred/checked types for functions and lets in `module`.
pub fn dump_types(module: &Module<'_>, analysis: &Analysis, interner: &Interner) -> String {
    let mut out = String::new();
    for item in module.items {
        if let Item::Fn(f) = item {
            let name = interner.resolve(f.name.name);
            let sig = analysis
                .scopes
                .lookup(analysis.module_scope, f.name.name)
                .and_then(|d| analysis.def_types.get(&d).copied())
                .map_or_else(
                    || "<unknown>".into(),
                    |t| {
                        analysis.types.display(
                            t,
                            &|d| analysis.defs[d.index() as usize].name(),
                            interner,
                        )
                    },
                );
            let _ = writeln!(out, "fn {name}: {sig}");
            dump_stmts(f.body, analysis, interner, &mut out, 1);
        }
    }
    out
}

fn dump_stmts(
    stmts: &[Stmt<'_>],
    analysis: &Analysis,
    interner: &Interner,
    out: &mut String,
    indent: usize,
) {
    let pad = "  ".repeat(indent);
    for stmt in stmts {
        match stmt {
            Stmt::Let(l) => {
                let name = interner.resolve(l.name.name);
                let ty = analysis.node_types.get(&l.id).map_or_else(
                    || "?".into(),
                    |t| {
                        analysis.types.display(
                            *t,
                            &|d| analysis.defs[d.index() as usize].name(),
                            interner,
                        )
                    },
                );
                let _ = writeln!(out, "{pad}let {name}: {ty}");
            }
            Stmt::Loop(l) => {
                let _ = writeln!(out, "{pad}loop:");
                dump_stmts(l.body, analysis, interner, out, indent + 1);
            }
            Stmt::Region(r) => {
                let _ = writeln!(out, "{pad}region:");
                dump_stmts(r.body, analysis, interner, out, indent + 1);
            }
            Stmt::For(f) => {
                let _ = writeln!(out, "{pad}for {}:", interner.resolve(f.binder.name));
                dump_stmts(f.body, analysis, interner, out, indent + 1);
            }
            Stmt::If(i) => {
                let _ = writeln!(out, "{pad}if:");
                for arm in i.arms {
                    dump_stmts(arm.body, analysis, interner, out, indent + 1);
                }
                if let Some(body) = i.else_body {
                    let _ = writeln!(out, "{pad}else:");
                    dump_stmts(body, analysis, interner, out, indent + 1);
                }
            }
            Stmt::Match(m) => {
                let _ = writeln!(out, "{pad}match:");
                for arm in m.arms {
                    dump_stmts(arm.body, analysis, interner, out, indent + 1);
                }
                if let Some(body) = m.else_body {
                    let _ = writeln!(out, "{pad}else:");
                    dump_stmts(body, analysis, interner, out, indent + 1);
                }
            }
            _ => {}
        }
    }
}
