//! Stdio LSP loop and document ingest.

use std::collections::HashMap;
use std::io::{self, BufRead, Read};

use serde_json::{json, Value};

use crate::lsp::diagnostics::{analyze_text, diag_to_lsp};
use crate::lsp::protocol::{uri_to_path, write_message, LspRequest};
use crate::lsp::Document;

pub struct LanguageServer {
    pub(crate) documents: HashMap<String, Document>,
}

impl LanguageServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn run_stdio(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        let mut stdout = io::stdout();

        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            if !line.starts_with("Content-Length: ") {
                continue;
            }
            let len: usize = line
                .trim_start_matches("Content-Length: ")
                .trim()
                .parse()
                .unwrap_or(0);
            let mut empty = String::new();
            let _ = reader.read_line(&mut empty);
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body)?;
            let Ok(v) = serde_json::from_slice::<Value>(&body) else {
                continue;
            };
            let req = LspRequest {
                id: v.get("id").cloned(),
                method: v
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or_default()
                    .to_string(),
                params: v.get("params").cloned(),
            };
            if let Some(resp) = self.handle_request(&req) {
                write_message(&mut stdout, &resp)?;
            }
            if req.method == "exit" {
                break;
            }
        }
        Ok(())
    }

    fn handle_request(&mut self, req: &LspRequest) -> Option<Value> {
        match req.method.as_str() {
            "initialize" => Some(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": {
                            "openClose": true,
                            "change": 1,
                            "save": { "includeText": false }
                        },
                        "definitionProvider": true,
                        "hoverProvider": true,
                        "completionProvider": {
                            "triggerCharacters": ["."]
                        },
                        "renameProvider": true,
                        "referencesProvider": true,
                        "workspaceSymbolProvider": true,
                        "documentSymbolProvider": true,
                        "documentFormattingProvider": true,
                        "codeActionProvider": {
                            "codeActionKinds": ["quickfix"]
                        }
                    },
                    "serverInfo": {
                        "name": "rynixc-lsp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            })),
            "initialized" | "textDocument/didSave" => None,
            "shutdown" => Some(json!({ "jsonrpc": "2.0", "id": req.id, "result": null })),
            "exit" => None,
            "textDocument/didOpen" => {
                self.ingest_open(req.params.as_ref());
                None
            }
            "textDocument/didChange" => {
                self.ingest_change(req.params.as_ref());
                None
            }
            "textDocument/definition" => Some(self.goto_definition(req)),
            "textDocument/hover" => Some(self.hover(req)),
            "textDocument/completion" => Some(self.completion(req)),
            "textDocument/rename" => Some(self.rename(req)),
            "textDocument/references" => Some(self.references(req)),
            "workspace/symbol" => Some(self.workspace_symbol(req)),
            "textDocument/documentSymbol" => Some(self.document_symbol(req)),
            "textDocument/formatting" => Some(self.formatting(req)),
            "textDocument/codeAction" => Some(self.code_action(req)),
            "textDocument/didClose" => {
                if let Some(params) = &req.params {
                    if let Some(uri) = params["textDocument"]["uri"].as_str() {
                        self.documents.remove(uri);
                    }
                }
                None
            }
            _ => Some(json!({ "jsonrpc": "2.0", "id": req.id, "result": null })),
        }
    }

    fn ingest_open(&mut self, params: Option<&Value>) {
        let Some(params) = params else { return };
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            return;
        };
        let Some(text) = params["textDocument"]["text"].as_str() else {
            return;
        };
        let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
        let path = uri_to_path(uri);
        self.documents.insert(
            uri.to_string(),
            Document {
                path,
                text: text.to_string(),
                version,
            },
        );
        self.publish_diagnostics(uri);
    }

    fn ingest_change(&mut self, params: Option<&Value>) {
        let Some(params) = params else { return };
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            return;
        };
        let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
        let Some(changes) = params["contentChanges"].as_array() else {
            return;
        };
        let Some(first) = changes.first() else { return };
        let Some(text) = first["text"].as_str() else { return };
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.text = text.to_string();
            doc.version = version;
        } else {
            let path = uri_to_path(uri);
            self.documents.insert(
                uri.to_string(),
                Document {
                    path,
                    text: text.to_string(),
                    version,
                },
            );
        }
        self.publish_diagnostics(uri);
    }

    fn publish_diagnostics(&self, uri: &str) {
        let Some(doc) = self.documents.get(uri) else { return };
        let mut stdout = io::stdout();
        let diags = analyze_text(&doc.path, &doc.text);
        let items: Vec<Value> = diags
            .into_iter()
            .filter_map(|d| diag_to_lsp(&d, uri))
            .collect();
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": items }
        });
        let _ = write_message(&mut stdout, &msg);
    }
}

pub fn run() -> std::process::ExitCode {
    let mut server = LanguageServer::new();
    if let Err(e) = server.run_stdio() {
        eprintln!("lsp error: {e}");
        return std::process::ExitCode::from(1);
    }
    std::process::ExitCode::SUCCESS
}

