//! `textDocument/inlayHint` — type hints at bindings from sema `def_types`.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::{DefId, DefKind, analyze};
use rynix_span::SourceMap;
use serde_json::{Value, json};

use crate::lsp::protocol::LspRequest;
use crate::lsp::server::LanguageServer;

impl LanguageServer {
    pub(crate) fn inlay_hint(&self, req: &LspRequest) -> Value {
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
        let analysis = analyze(module, &mut interner, &mut sink);
        let mut sources = SourceMap::new();
        let label = doc.path.to_string_lossy();
        sources.add_owned(label.as_ref(), doc.text.clone());
        let file_start = sources.files().next().map(|f| f.start_pos()).unwrap_or(0);

        let resolve = |d: DefId| analysis.defs[d.index() as usize].name();
        let mut hints = Vec::new();
        for (&def_id, &ty) in &analysis.def_types {
            let def = &analysis.defs[def_id.index() as usize];
            let span = match def {
                DefKind::Local { span, .. } | DefKind::Param { span, .. } => *span,
                _ => continue,
            };
            let hi = span.hi().saturating_sub(file_start) as usize;
            if hi < doc.text.len() {
                let after = doc.text[hi..].trim_start();
                if after.starts_with(':') {
                    continue;
                }
            }
            let ty_label = analysis.types.display(ty, &resolve, &interner);
            let (_f, end) = sources.line_col(span.hi());
            hints.push(json!({
                "position": {
                    "line": end.line.saturating_sub(1),
                    "character": end.col.saturating_sub(1)
                },
                "label": format!(": {ty_label}"),
                "kind": 1,
                "paddingLeft": true
            }));
        }
        json!({ "jsonrpc": "2.0", "id": req.id, "result": hints })
    }
}
