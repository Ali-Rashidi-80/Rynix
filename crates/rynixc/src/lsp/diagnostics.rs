//! Compile-and-map diagnostics for open documents.

use std::path::Path;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::SourceMap;
use serde_json::{json, Value};

pub(crate) struct ParsedDiag {
    pub(crate) severity: u8,
    pub(crate) message: String,
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) end_line: u32,
    pub(crate) end_col: u32,
}

pub(crate) fn analyze_text(path: &Path, text: &str) -> Vec<ParsedDiag> {
    let mut sources = SourceMap::new();
    let name = path.to_string_lossy();
    sources.add_owned(name.as_ref(), text.to_string());
    let file = sources.files().next().expect("one file");
    let arena = AstArena::new();
    let mut interner = rynix_span::Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(
        &arena,
        &mut interner,
        file.text(),
        file.start_pos(),
        &mut sink,
    );
    let _ = analyze(module, &mut interner, &mut sink);

    sink.diags
        .iter()
        .filter_map(|d| {
            let (_f, start) = sources.line_col(d.primary.span.lo());
            let (_, end) = sources.line_col(d.primary.span.hi());
            Some(ParsedDiag {
                severity: match d.severity {
                    rynix_diag::Severity::Error => 1,
                    rynix_diag::Severity::Warning => 2,
                    rynix_diag::Severity::Note => 3,
                    rynix_diag::Severity::Help => 4,
                },
                message: d.message.clone(),
                line: start.line,
                col: start.col,
                end_line: end.line,
                end_col: end.col,
            })
        })
        .collect()
}

pub(crate) fn diag_to_lsp(d: &ParsedDiag, uri: &str) -> Option<Value> {
    Some(json!({
        "range": {
            "start": { "line": d.line.saturating_sub(1), "character": d.col.saturating_sub(1) },
            "end": { "line": d.end_line.saturating_sub(1), "character": d.end_col.saturating_sub(1) }
        },
        "severity": d.severity,
        "source": "rynixc",
        "message": d.message,
        "uri": uri
    }))
}

