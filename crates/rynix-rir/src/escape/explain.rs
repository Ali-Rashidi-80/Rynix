//! Human and JSON explainers for allocation placement.

use std::fmt::Write as _;

use rynix_span::Interner;
use serde_json::json;

use crate::ir::Module;

use super::analyze::EscapeReport;

/// Multi-line human explanation of every allocation site.
pub fn explain_alloc_human(module: &Module, report: &EscapeReport, interner: &Interner) -> String {
    let mut out = String::new();
    for site in &report.sites {
        let fname = module
            .funcs
            .get(site.func.0 as usize)
            .map_or("?", |f| interner.resolve(f.name));
        let _ = writeln!(
            out,
            "site{} @{fname}: {} ({}) — {}",
            site.site.0,
            site.placement.as_str(),
            site.escape.as_str(),
            site.reason
        );
        if let Some(at) = site.free_at {
            let _ = writeln!(out, "  free-at: inst{at}");
        }
    }
    if out.is_empty() {
        out.push_str("(no allocation sites)\n");
    }
    out
}

/// One JSON object per line (agent-friendly), not full rynix.diag.v1.
pub fn explain_alloc_json(module: &Module, report: &EscapeReport, interner: &Interner) -> String {
    let mut out = String::new();
    for site in &report.sites {
        let fname = module
            .funcs
            .get(site.func.0 as usize)
            .map_or("?", |f| interner.resolve(f.name));
        let obj = json!({
            "schema": "rynix.alloc.v1",
            "func": fname,
            "site": site.site.0,
            "escape": site.escape.as_str(),
            "placement": site.placement.as_str(),
            "reason": site.reason,
            "span": { "lo": site.span.lo(), "hi": site.span.hi() },
            "free_at": site.free_at,
        });
        out.push_str(&obj.to_string());
        out.push('\n');
    }
    out
}
