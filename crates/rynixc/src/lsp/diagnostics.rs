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
    /// LSP TextEdits for each attached compiler Fix (empty if none).
    pub(crate) fix_actions: Vec<ParsedFix>,
}

pub(crate) struct ParsedFix {
    pub(crate) title: String,
    pub(crate) confidence: f32,
    pub(crate) edits: Vec<ParsedTextEdit>,
}

pub(crate) struct ParsedTextEdit {
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) end_line: u32,
    pub(crate) end_col: u32,
    pub(crate) new_text: String,
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
            let fix_actions = d
                .fixes
                .iter()
                .map(|fix| {
                    let edits = fix
                        .edits
                        .iter()
                        .map(|e| {
                            let (_f, es) = sources.line_col(e.span.lo());
                            let (_, ee) = sources.line_col(e.span.hi());
                            ParsedTextEdit {
                                line: es.line,
                                col: es.col,
                                end_line: ee.line,
                                end_col: ee.col,
                                new_text: e.replacement.clone(),
                            }
                        })
                        .collect();
                    ParsedFix {
                        title: fix.message.clone(),
                        confidence: fix.confidence,
                        edits,
                    }
                })
                .collect();
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
                fix_actions,
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

fn pos_le(a: (u32, u32), b: (u32, u32)) -> bool {
    a <= b
}

fn ranges_overlap(a0: (u32, u32), a1: (u32, u32), b0: (u32, u32), b1: (u32, u32)) -> bool {
    pos_le(a0, b1) && pos_le(b0, a1)
}

/// Build `textDocument/codeAction` result items from compiler Fixes.
pub(crate) fn code_actions_for_doc(
    path: &Path,
    text: &str,
    uri: &str,
    range_start_line: u32,
    range_start_col: u32,
    range_end_line: u32,
    range_end_col: u32,
) -> Vec<Value> {
    let diags = analyze_text(path, text);
    let req0 = (range_start_line, range_start_col);
    let req1 = (range_end_line, range_end_col);
    let mut out = Vec::new();
    for d in diags {
        for fix in &d.fix_actions {
            if fix.edits.is_empty() {
                continue;
            }
            let diag0 = (d.line.saturating_sub(1), d.col.saturating_sub(1));
            let diag1 = (d.end_line.saturating_sub(1), d.end_col.saturating_sub(1));
            let diag_overlap = ranges_overlap(diag0, diag1, req0, req1);
            let edit_overlap = fix.edits.iter().any(|e| {
                ranges_overlap(
                    (e.line.saturating_sub(1), e.col.saturating_sub(1)),
                    (e.end_line.saturating_sub(1), e.end_col.saturating_sub(1)),
                    req0,
                    req1,
                )
            });
            if !diag_overlap && !edit_overlap {
                continue;
            }
            let edits: Vec<Value> = fix
                .edits
                .iter()
                .map(|e| {
                    json!({
                        "range": {
                            "start": {
                                "line": e.line.saturating_sub(1),
                                "character": e.col.saturating_sub(1)
                            },
                            "end": {
                                "line": e.end_line.saturating_sub(1),
                                "character": e.end_col.saturating_sub(1)
                            }
                        },
                        "newText": e.new_text
                    })
                })
                .collect();
            out.push(json!({
                "title": fix.title,
                "kind": "quickfix",
                "isPreferred": fix.confidence >= 0.9,
                "edit": {
                    "changes": {
                        uri: edits
                    }
                }
            }));
        }
    }
    out
}
