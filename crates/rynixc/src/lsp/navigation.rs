//! Go-to-definition and hover handlers.

use std::fs;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::SourceMap;
use serde_json::{json, Value};

use crate::lsp::protocol::{path_to_uri, pos_from_line_col, LspRequest};
use crate::lsp::resolve::{find_definition_span, find_workspace_fn_def, hover_at, name_at_offset};
use crate::lsp::server::LanguageServer;

impl LanguageServer {
    pub(crate) fn goto_definition(&self, req: &LspRequest) -> Value {
        let empty = json!(null);
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

        let result = if let Some(span) = find_definition_span(module, &analysis, offset) {
            let (_f, start) = sources.line_col(span.lo());
            let (_, end) = sources.line_col(span.hi());
            Some(json!({
                "uri": uri,
                "range": {
                    "start": { "line": start.line.saturating_sub(1), "character": start.col.saturating_sub(1) },
                    "end": { "line": end.line.saturating_sub(1), "character": end.col.saturating_sub(1) }
                }
            }))
        } else if let Some(name) = name_at_offset(module, &interner, offset) {
            // L12: resolve from on-disk workspace member sources via manifest.
            find_workspace_fn_def(&doc.path, &name).map(|(path, span)| {
                let mut ws_sources = SourceMap::new();
                let label = path.to_string_lossy();
                let text = fs::read_to_string(&path).unwrap_or_default();
                ws_sources.add_owned(label.as_ref(), text);
                let (_f, start) = ws_sources.line_col(span.lo());
                let (_, end) = ws_sources.line_col(span.hi());
                json!({
                    "uri": path_to_uri(&path),
                    "range": {
                        "start": { "line": start.line.saturating_sub(1), "character": start.col.saturating_sub(1) },
                        "end": { "line": end.line.saturating_sub(1), "character": end.col.saturating_sub(1) }
                    }
                })
            })
        } else {
            None
        };

        json!({ "jsonrpc": "2.0", "id": req.id, "result": result })
    }

    pub(crate) fn hover(&self, req: &LspRequest) -> Value {
        let empty = json!(null);
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

        let hover_text = hover_at(module, &analysis, &interner, offset);
        let result = hover_text.map(|text| {
            json!({
                "contents": { "kind": "markdown", "value": format!("```rynix\n{text}\n```") }
            })
        });

        json!({ "jsonrpc": "2.0", "id": req.id, "result": result })
    }
}
