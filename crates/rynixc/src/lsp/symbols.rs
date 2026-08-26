//! Workspace and document symbol providers.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::{analyze, DefKind};
use rynix_span::SourceMap;
use serde_json::{json, Value};

use crate::lsp::protocol::LspRequest;
use crate::lsp::server::LanguageServer;

impl LanguageServer {
    pub(crate) fn workspace_symbol(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let query = params["query"].as_str().unwrap_or("").to_ascii_lowercase();
        let mut symbols = Vec::new();
        for (uri, doc) in &self.documents {
            let arena = AstArena::new();
            let mut interner = rynix_span::Interner::new();
            let mut sink = VecSink::new();
            let module = rynix_parser::parse(
                &arena,
                &mut interner,
                &doc.text,
                0,
                &mut sink,
            );
            let analysis = analyze(module, &mut interner, &mut sink);
            let mut sources = SourceMap::new();
            let label = doc.path.to_string_lossy();
            sources.add_owned(label.as_ref(), doc.text.clone());
            for def in &analysis.defs {
                let (kind, name_sym, span) = match def {
                    DefKind::Fn { name, span, .. } => (12u8, *name, *span), // Function
                    DefKind::Struct { name, span, .. } => (23u8, *name, *span), // Struct
                    DefKind::Enum { name, span, .. } => (10u8, *name, *span), // Enum
                    DefKind::Variant { name, span, .. } => (22u8, *name, *span), // EnumMember
                    _ => continue,
                };
                let name = interner.resolve(name_sym).to_string();
                if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let (_f, start) = sources.line_col(span.lo());
                let (_, end) = sources.line_col(span.hi());
                symbols.push(json!({
                    "name": name,
                    "kind": kind,
                    "location": {
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
                    }
                }));
            }
        }
        symbols.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        json!({ "jsonrpc": "2.0", "id": req.id, "result": symbols })
    }

    pub(crate) fn document_symbol(&self, req: &LspRequest) -> Value {
        let empty = json!([]);
        let Some(params) = &req.params else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "jsonrpc": "2.0", "id": req.id, "result": empty });
        };
        let arena = AstArena::new();
        let mut interner = rynix_span::Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(&arena, &mut interner, &doc.text, 0, &mut sink);
        let analysis = analyze(module, &mut interner, &mut sink);
        let mut sources = SourceMap::new();
        let label = doc.path.to_string_lossy();
        sources.add_owned(label.as_ref(), doc.text.clone());
        let mut symbols = Vec::new();
        for def in &analysis.defs {
            let (kind, name_sym, span) = match def {
                DefKind::Fn { name, span, .. } => (12u8, *name, *span),
                DefKind::Struct { name, span, .. } => (23u8, *name, *span),
                DefKind::Enum { name, span, .. } => (10u8, *name, *span),
                DefKind::Variant { name, span, .. } => (22u8, *name, *span),
                _ => continue,
            };
            let name = interner.resolve(name_sym).to_string();
            let (_f, start) = sources.line_col(span.lo());
            let (_, end) = sources.line_col(span.hi());
            let range = json!({
                "start": {
                    "line": start.line.saturating_sub(1),
                    "character": start.col.saturating_sub(1)
                },
                "end": {
                    "line": end.line.saturating_sub(1),
                    "character": end.col.saturating_sub(1)
                }
            });
            symbols.push(json!({
                "name": name,
                "kind": kind,
                "range": range,
                "selectionRange": range
            }));
        }
        symbols.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        json!({ "jsonrpc": "2.0", "id": req.id, "result": symbols })
    }
}
