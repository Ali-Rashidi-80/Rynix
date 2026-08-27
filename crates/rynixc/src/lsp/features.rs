//! Completion, rename, references, formatting, and codeAction handlers.

use rynix_ast::{format_module, AstArena};
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::SourceMap;
use serde_json::{json, Value};

use crate::lsp::diagnostics::code_actions_for_doc;
use crate::lsp::protocol::{pos_from_line_col, LspRequest};
use crate::lsp::resolve::{
    completion_items, completion_prefix, def_index_at, is_ident, reference_spans,
};
use crate::lsp::server::LanguageServer;

impl LanguageServer {
    pub(crate) fn completion(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        let line = params["position"]["line"].as_u64().unwrap_or(0) as u32 + 1;
        let col = params["position"]["character"].as_u64().unwrap_or(0) as u32 + 1;
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };

        let mut sources = SourceMap::new();
        let label = doc.path.to_string_lossy();
        sources.add_owned(label.as_ref(), doc.text.clone());
        let file = sources.files().next().expect("one file");
        let offset = pos_from_line_col(file, line, col);

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
        let analysis = analyze(module, &mut interner, &mut sink);

        let prefix = completion_prefix(&doc.text, file.start_pos(), offset);
        let items = completion_items(&analysis, &interner, offset, prefix.as_deref());
        json!({ "jsonrpc": "2.0", "id": req.id, "result": items })
    }

    pub(crate) fn rename(&self, req: &LspRequest) -> Value {
        let empty = json!(null);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        let new_name = params["newName"].as_str().unwrap_or_default();
        if new_name.is_empty() || !is_ident(new_name) {
            return json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "error": { "code": -32602, "message": "invalid newName" }
            });
        }
        let line = params["position"]["line"].as_u64().unwrap_or(0) as u32 + 1;
        let col = params["position"]["character"].as_u64().unwrap_or(0) as u32 + 1;
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };

        let mut sources = SourceMap::new();
        let label = doc.path.to_string_lossy();
        sources.add_owned(label.as_ref(), doc.text.clone());
        let file = sources.files().next().expect("one file");
        let offset = pos_from_line_col(file, line, col);

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
        let analysis = analyze(module, &mut interner, &mut sink);

        let Some(def_idx) = def_index_at(module, &analysis, offset) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let spans = reference_spans(module, &analysis, def_idx);
        if spans.is_empty() {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        }

        let mut edits: Vec<Value> = spans
            .into_iter()
            .map(|span| {
                let (_f, start) = sources.line_col(span.lo());
                let (_, end) = sources.line_col(span.hi());
                json!({
                    "range": {
                        "start": {
                            "line": start.line.saturating_sub(1),
                            "character": start.col.saturating_sub(1)
                        },
                        "end": {
                            "line": end.line.saturating_sub(1),
                            "character": end.col.saturating_sub(1)
                        }
                    },
                    "newText": new_name
                })
            })
            .collect();
        // Stable order: later edits first so clients applying sequentially stay valid.
        edits.sort_by(|a, b| {
            let al = a["range"]["start"]["line"].as_u64().unwrap_or(0);
            let bl = b["range"]["start"]["line"].as_u64().unwrap_or(0);
            bl.cmp(&al).then_with(|| {
                let ac = a["range"]["start"]["character"].as_u64().unwrap_or(0);
                let bc = b["range"]["start"]["character"].as_u64().unwrap_or(0);
                bc.cmp(&ac)
            })
        });

        json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": {
                "changes": { uri: edits }
            }
        })
    }

    pub(crate) fn references(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        let include_decl = params["context"]["includeDeclaration"]
            .as_bool()
            .unwrap_or(true);
        let line = params["position"]["line"].as_u64().unwrap_or(0) as u32 + 1;
        let col = params["position"]["character"].as_u64().unwrap_or(0) as u32 + 1;
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };

        let mut sources = SourceMap::new();
        let label = doc.path.to_string_lossy();
        sources.add_owned(label.as_ref(), doc.text.clone());
        let file = sources.files().next().expect("one file");
        let offset = pos_from_line_col(file, line, col);

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
        let analysis = analyze(module, &mut interner, &mut sink);

        let Some(def_idx) = def_index_at(module, &analysis, offset) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let def_span = analysis.defs.get(def_idx).and_then(|d| d.span());
        let mut locs = Vec::new();
        for span in reference_spans(module, &analysis, def_idx) {
            if !include_decl {
                if let Some(ds) = def_span {
                    if span.lo() == ds.lo() && span.hi() == ds.hi() {
                        continue;
                    }
                }
            }
            let (_f, start) = sources.line_col(span.lo());
            let (_, end) = sources.line_col(span.hi());
            locs.push(json!({
                "uri": uri,
                "range": {
                    "start": {
                        "line": start.line.saturating_sub(1),
                        "character": start.col.saturating_sub(1)
                    },
                    "end": {
                        "line": end.line.saturating_sub(1),
                        "character": end.col.saturating_sub(1)
                    }
                }
            }));
        }
        json!({ "jsonrpc": "2.0", "id": req.id, "result": locs })
    }

    /// `textDocument/formatting` — full-document edit via `format_module` (same as `rynixc fmt`).
    pub(crate) fn formatting(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };

        let arena = AstArena::new();
        let mut interner = rynix_span::Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(&arena, &mut interner, &doc.text, 0, &mut sink);
        if sink.error_count() > 0 {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        }
        let formatted = format_module(module, &interner, &doc.text, 0);
        if formatted == doc.text {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        }

        let (end_line, end_col) = lsp_end_position(&doc.text);
        json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "result": [{
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": end_line, "character": end_col }
                },
                "newText": formatted
            }]
        })
    }

    /// `textDocument/codeAction` — expose compiler `Diagnostic.fixes` as quickfixes.
    pub(crate) fn code_action(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let start_line = params["range"]["start"]["line"].as_u64().unwrap_or(0) as u32;
        let start_col = params["range"]["start"]["character"].as_u64().unwrap_or(0) as u32;
        let end_line = params["range"]["end"]["line"].as_u64().unwrap_or(start_line as u64) as u32;
        let end_col = params["range"]["end"]["character"].as_u64().unwrap_or(start_col as u64) as u32;
        let actions = code_actions_for_doc(
            &doc.path,
            &doc.text,
            uri,
            start_line,
            start_col,
            end_line,
            end_col,
        );
        json!({ "jsonrpc": "2.0", "id": req.id, "result": actions })
    }
}

fn lsp_end_position(text: &str) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}
