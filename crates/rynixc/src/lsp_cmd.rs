//! Minimal LSP server on stdio: full-sync documents, diagnostics, go-to-definition,
//! hover, completion, rename, references, and workspace symbols.

#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms,
    clippy::needless_borrow,
    clippy::single_match,
    clippy::too_many_lines,
    clippy::unnecessary_filter_map,
    clippy::unnecessary_wraps
)]

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

use rynix_ast::{Expr, Item, Module, Path as AstPath, Stmt};
use rynix_ast::{AstArena, Ident};
use rynix_diag::VecSink;
use rynix_sema::{analyze, Analysis, DefKind};
use rynix_span::{SourceMap, Span};
use serde_json::{json, Value};

use crate::manifest::{find_workspace_root, load_manifest};

struct LspRequest {
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

struct Document {
    path: PathBuf,
    text: String,
    version: i64,
}

pub struct LanguageServer {
    documents: HashMap<String, Document>,
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
                        "workspaceSymbolProvider": true
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

    fn goto_definition(&self, req: &LspRequest) -> Value {
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

    fn hover(&self, req: &LspRequest) -> Value {
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

    fn completion(&self, req: &LspRequest) -> Value {
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

    fn rename(&self, req: &LspRequest) -> Value {
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

    fn references(&self, req: &LspRequest) -> Value {
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

    fn workspace_symbol(&self, req: &LspRequest) -> Value {
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
}

pub fn run() -> std::process::ExitCode {
    let mut server = LanguageServer::new();
    if let Err(e) = server.run_stdio() {
        eprintln!("lsp error: {e}");
        return std::process::ExitCode::from(1);
    }
    std::process::ExitCode::SUCCESS
}

fn write_message(out: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    out.flush()
}

fn uri_to_path(uri: &str) -> PathBuf {
    if let Some(rest) = uri.strip_prefix("file://") {
        let path = rest.trim_start_matches('/');
        if rest.starts_with('/') && path.len() >= 2 && path.as_bytes()[1] == b':' {
            // Windows file:///C:/...
            PathBuf::from(path)
        } else if cfg!(windows) && path.contains(':') {
            PathBuf::from(path)
        } else {
            PathBuf::from(format!("/{path}"))
        }
    } else {
        PathBuf::from(uri)
    }
}

fn path_to_uri(path: &Path) -> String {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        format!("file:///{s}")
    } else {
        format!("file://{s}")
    }
}

/// Identifier / path segment name under `offset` (for workspace fallback).
fn name_at_offset(
    module: &Module<'_>,
    interner: &rynix_span::Interner,
    offset: u32,
) -> Option<String> {
    if let Some(path) = find_path_at(module, offset) {
        if let Some(seg) = path
            .segments
            .iter()
            .find(|s| s.span.contains(offset))
            .or_else(|| path.segments.last())
        {
            return Some(interner.resolve(seg.name).to_string());
        }
    }
    find_ident_at(module, offset).map(|id| interner.resolve(id.name).to_string())
}

/// On-disk workspace member sources: `[package].entry` then `files` for each member.
fn workspace_member_sources(from: &Path) -> Vec<PathBuf> {
    let Some(ws_root) = find_workspace_root(from) else {
        return Vec::new();
    };
    let ws_toml = ws_root.join("rynix.toml");
    let Ok(ws) = load_manifest(&ws_toml) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for rel in &ws.workspace_members {
        let member_dir = ws_root.join(rel);
        let member_toml = member_dir.join("rynix.toml");
        let Ok(m) = load_manifest(&member_toml) else {
            continue;
        };
        if let Some(entry) = &m.entry {
            let p = member_dir.join(entry);
            if p.is_file() {
                out.push(p);
            }
        }
        for f in &m.files {
            let p = member_dir.join(f);
            if p.is_file() && !out.iter().any(|e| e == &p) {
                out.push(p);
            }
        }
    }
    out
}

/// Find `def <name>` in a source file; returns the name span (file-local SourceMap).
fn find_fn_def_span_in_file(path: &Path, name: &str) -> Option<Span> {
    let text = fs::read_to_string(path).ok()?;
    let arena = AstArena::new();
    let mut interner = rynix_span::Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, &text, 0, &mut sink);
    for item in module.items {
        if let Item::Fn(f) = item {
            if interner.resolve(f.name.name) == name {
                return Some(f.name.span);
            }
        }
    }
    None
}

/// Resolve a function definition from workspace member sources on disk (L12).
fn find_workspace_fn_def(doc_path: &Path, name: &str) -> Option<(PathBuf, Span)> {
    let doc_canon = fs::canonicalize(doc_path).unwrap_or_else(|_| doc_path.to_path_buf());
    for path in workspace_member_sources(doc_path) {
        let path_canon = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if path_canon == doc_canon {
            continue;
        }
        if let Some(span) = find_fn_def_span_in_file(&path, name) {
            return Some((path, span));
        }
    }
    None
}

fn pos_from_line_col(file: &rynix_span::SourceFile, line: u32, col: u32) -> u32 {
    let line = line.max(1);
    let col = col.max(1);
    let mut local = 0u32;
    for l in 1..line {
        local += file.line_text(l).len() as u32 + 1;
    }
    local += col.saturating_sub(1).min(file.line_text(line).len() as u32);
    file.start_pos().saturating_add(local)
}

struct ParsedDiag {
    severity: u8,
    message: String,
    line: u32,
    col: u32,
    end_line: u32,
    end_col: u32,
}

fn analyze_text(path: &Path, text: &str) -> Vec<ParsedDiag> {
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

fn diag_to_lsp(d: &ParsedDiag, uri: &str) -> Option<Value> {
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

fn find_definition_span(module: &Module<'_>, analysis: &Analysis, offset: u32) -> Option<Span> {
    if let Some(path) = find_path_at(module, offset) {
        if let Some(def_id) = analysis.path_resolution.get(&path.id) {
            let def = &analysis.defs[def_id.index() as usize];
            if let Some(span) = def.span() {
                return Some(span);
            }
        }
    }
    if let Some(ident) = find_ident_at(module, offset) {
        for def in &analysis.defs {
            if def.name() == ident.name {
                if let Some(span) = def.span() {
                    if span.contains(offset) || ident.span.contains(offset) {
                        return Some(ident.span);
                    }
                }
            }
        }
    }
    None
}

fn hover_at(
    module: &Module<'_>,
    analysis: &Analysis,
    interner: &rynix_span::Interner,
    offset: u32,
) -> Option<String> {
    let node = find_expr_at(module, offset)?;
    let ty = analysis.node_types.get(&node)?;
    let resolve = |d: rynix_sema::DefId| analysis.defs[d.index() as usize].name();
    Some(analysis.types.display(*ty, &resolve, interner))
}

/// Trailing identifier fragment before `offset` (for filtering completions).
fn completion_prefix(text: &str, file_start: u32, offset: u32) -> Option<String> {
    let local = offset.saturating_sub(file_start) as usize;
    if local == 0 || local > text.len() {
        return None;
    }
    let before = &text[..local];
    let start = before
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let prefix = &before[start..];
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// Local/module function and let (and param) bindings visible near `offset`.
fn completion_items(
    analysis: &Analysis,
    interner: &rynix_span::Interner,
    offset: u32,
    prefix: Option<&str>,
) -> Vec<Value> {
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for def in &analysis.defs {
        let (kind, detail, near) = match def {
            DefKind::Fn { .. } => (3u8, "fn", true), // CompletionItemKind.Function
            DefKind::Local { span, .. } => (6u8, "let", span.lo() <= offset),
            DefKind::Param { span, .. } => (6u8, "param", span.lo() <= offset),
            _ => continue,
        };
        if !near {
            continue;
        }
        let label = interner.resolve(def.name()).to_string();
        if let Some(p) = prefix {
            if !label.starts_with(p) {
                continue;
            }
        }
        if !seen.insert(label.clone()) {
            continue;
        }
        items.push(json!({
            "label": label,
            "kind": kind,
            "detail": detail,
            "insertText": label,
        }));
    }
    items.sort_by(|a, b| {
        a["label"]
            .as_str()
            .unwrap_or("")
            .cmp(b["label"].as_str().unwrap_or(""))
    });
    items
}

fn def_index_at(module: &Module<'_>, analysis: &Analysis, offset: u32) -> Option<usize> {
    if let Some(path) = find_path_at(module, offset) {
        if let Some(def_id) = analysis.path_resolution.get(&path.id) {
            let idx = def_id.index() as usize;
            if renameable_def(&analysis.defs[idx]) {
                return Some(idx);
            }
        }
    }
    for (i, def) in analysis.defs.iter().enumerate() {
        if !renameable_def(def) {
            continue;
        }
        if let Some(span) = def.span() {
            if span.contains(offset) {
                return Some(i);
            }
        }
    }
    None
}

fn renameable_def(def: &DefKind) -> bool {
    matches!(
        def,
        DefKind::Fn { .. } | DefKind::Local { .. } | DefKind::Param { .. }
    )
}

fn reference_spans(module: &Module<'_>, analysis: &Analysis, def_idx: usize) -> Vec<Span> {
    let mut spans = Vec::new();
    if let Some(span) = analysis.defs.get(def_idx).and_then(|d| d.span()) {
        spans.push(span);
    }
    for item in module.items {
        walk_item(item, &mut |path: &AstPath| {
            if analysis
                .path_resolution
                .get(&path.id)
                .map(|d| d.index() as usize)
                == Some(def_idx)
            {
                if let Some(seg) = path.segments.last() {
                    if !spans.iter().any(|s| s.lo() == seg.span.lo() && s.hi() == seg.span.hi())
                    {
                        spans.push(seg.span);
                    }
                }
            }
        });
    }
    spans
}

fn find_expr_at(module: &Module<'_>, offset: u32) -> Option<rynix_ast::NodeId> {
    let mut best: Option<(u32, rynix_ast::NodeId)> = None;
    for item in module.items {
        if let Item::Fn(f) = item {
            for stmt in f.body {
                walk_stmt_for_expr(stmt, offset, &mut best);
            }
        }
    }
    best.map(|(_, id)| id)
}

fn consider_expr(expr: &Expr<'_>, offset: u32, best: &mut Option<(u32, rynix_ast::NodeId)>) {
    if !expr.span().contains(offset) {
        return;
    }
    let len = expr.span().len();
    if best.as_ref().is_none_or(|(l, _)| len <= *l) {
        *best = Some((len, expr.id()));
    }
    match expr {
        Expr::Binary(b) => {
            consider_expr(b.lhs, offset, best);
            consider_expr(b.rhs, offset, best);
        }
        Expr::Unary(u) => consider_expr(u.operand, offset, best),
        Expr::Call(c) => {
            consider_expr(c.callee, offset, best);
            for a in c.args {
                consider_expr(a, offset, best);
            }
        }
        Expr::MethodCall(m) => {
            consider_expr(m.receiver, offset, best);
            for a in m.args {
                consider_expr(a, offset, best);
            }
        }
        Expr::Field(f) => consider_expr(f.base, offset, best),
        Expr::StructLit(s) => {
            for init in s.fields {
                consider_expr(init.value, offset, best);
            }
        }
        Expr::Index(i) => {
            consider_expr(i.base, offset, best);
            consider_expr(i.index, offset, best);
        }
        Expr::Cast(c) => consider_expr(c.expr, offset, best),
        Expr::Array(a) => {
            for e in a.elems {
                consider_expr(e, offset, best);
            }
        }
        Expr::Spawn(s) => consider_expr(s.callee, offset, best),
        Expr::Path(_) | Expr::Literal(_) | Expr::Error(_) => {}
    }
}

fn walk_stmt_for_expr(stmt: &Stmt<'_>, offset: u32, best: &mut Option<(u32, rynix_ast::NodeId)>) {
    match stmt {
        Stmt::Let(l) => consider_expr(l.init, offset, best),
        Stmt::Expr(e) => consider_expr(&e.expr, offset, best),
        Stmt::If(i) => {
            for arm in i.arms {
                consider_expr(arm.cond, offset, best);
                for s in arm.body {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
            if let Some(el) = i.else_body {
                for s in el {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
        }
        Stmt::Match(m) => {
            consider_expr(m.scrutinee, offset, best);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt_for_expr(s, offset, best);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::For(f) => {
            consider_expr(f.iter, offset, best);
            for s in f.body {
                walk_stmt_for_expr(s, offset, best);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                consider_expr(e, offset, best);
            }
        }
        Stmt::Assign(a) => {
            consider_expr(a.target, offset, best);
            consider_expr(a.value, offset, best);
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
    }
}

fn find_path_at<'a>(module: &Module<'a>, offset: u32) -> Option<&'a AstPath<'a>> {
    let mut found: Option<&AstPath> = None;
    for item in module.items {
        walk_item(item, &mut |path: &AstPath| {
            if path.span.contains(offset) || path_segment_contains(path, offset) {
                found = Some(path);
            }
        });
    }
    found
}

fn path_segment_contains(path: &AstPath, offset: u32) -> bool {
    path.segments
        .iter()
        .any(|s| s.span.contains(offset))
}

fn find_ident_at<'a>(module: &Module<'a>, offset: u32) -> Option<Ident> {
    let mut found = None;
    for item in module.items {
        walk_item_idents(item, offset, &mut found);
    }
    found
}

fn walk_item_idents(item: &Item<'_>, offset: u32, found: &mut Option<Ident>) {
    match item {
        Item::Fn(f) => {
            if f.name.span.contains(offset) {
                *found = Some(f.name);
            }
            for p in f.params {
                if p.name.span.contains(offset) {
                    *found = Some(p.name);
                }
            }
            for stmt in f.body {
                walk_stmt_idents(stmt, offset, found);
            }
        }
        Item::Struct(s) => {
            if s.name.span.contains(offset) {
                *found = Some(s.name);
            }
        }
        Item::Enum(e) => {
            if e.name.span.contains(offset) {
                *found = Some(e.name);
            }
            for v in e.variants {
                if v.name.span.contains(offset) {
                    *found = Some(v.name);
                }
            }
        }
        Item::TypeAlias(t) => {
            if t.name.span.contains(offset) {
                *found = Some(t.name);
            }
        }
        _ => {}
    }
}

fn walk_stmt_idents(stmt: &Stmt<'_>, offset: u32, found: &mut Option<Ident>) {
    match stmt {
        Stmt::Let(l) => {
            if l.name.span.contains(offset) {
                *found = Some(l.name);
            }
            walk_expr_idents(l.init, offset, found);
        }
        Stmt::Expr(e) => walk_expr_idents(&e.expr, offset, found),
        Stmt::If(i) => walk_if_idents(i, offset, found),
        Stmt::Match(m) => {
            walk_expr_idents(m.scrutinee, offset, found);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt_idents(s, offset, found);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt_idents(s, offset, found);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::For(f) => {
            if f.binder.span.contains(offset) {
                *found = Some(f.binder);
            }
            walk_expr_idents(f.iter, offset, found);
            for s in f.body {
                walk_stmt_idents(s, offset, found);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                walk_expr_idents(e, offset, found);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        Stmt::Assign(a) => {
            walk_expr_idents(a.target, offset, found);
            walk_expr_idents(a.value, offset, found);
        }
    }
}

fn walk_if_idents(i: &rynix_ast::IfStmt<'_>, offset: u32, found: &mut Option<Ident>) {
    for arm in i.arms {
        walk_expr_idents(arm.cond, offset, found);
        for s in arm.body {
            walk_stmt_idents(s, offset, found);
        }
    }
    if let Some(el) = i.else_body {
        for s in el {
            walk_stmt_idents(s, offset, found);
        }
    }
}

fn walk_expr_idents(expr: &Expr<'_>, offset: u32, found: &mut Option<Ident>) {
    match expr {
        Expr::Path(p) => {
            for seg in p.segments {
                if seg.span.contains(offset) {
                    *found = Some(*seg);
                }
            }
        }
        Expr::Binary(b) => {
            walk_expr_idents(b.lhs, offset, found);
            walk_expr_idents(b.rhs, offset, found);
        }
        Expr::Unary(u) => walk_expr_idents(u.operand, offset, found),
        Expr::Call(c) => {
            walk_expr_idents(c.callee, offset, found);
            for a in c.args {
                walk_expr_idents(a, offset, found);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr_idents(m.receiver, offset, found);
            for a in m.args {
                walk_expr_idents(a, offset, found);
            }
        }
        Expr::Field(f) => walk_expr_idents(f.base, offset, found),
        Expr::StructLit(s) => {
            for seg in s.path.segments {
                if seg.span.contains(offset) {
                    *found = Some(*seg);
                }
            }
            for init in s.fields {
                if init.name.span.contains(offset) {
                    *found = Some(init.name);
                }
                walk_expr_idents(init.value, offset, found);
            }
        }
        Expr::Index(i) => {
            walk_expr_idents(i.base, offset, found);
            walk_expr_idents(i.index, offset, found);
        }
        Expr::Cast(c) => walk_expr_idents(c.expr, offset, found),
        Expr::Array(a) => {
            for e in a.elems {
                walk_expr_idents(e, offset, found);
            }
        }
        Expr::Spawn(s) => walk_expr_idents(s.callee, offset, found),
        Expr::Literal(_) | Expr::Error(_) => {}
    }
}

fn walk_item<'a>(item: &Item<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match item {
        Item::Fn(f) => {
            for stmt in f.body {
                walk_stmt(stmt, on_path);
            }
        }
        _ => {}
    }
}

fn walk_stmt<'a>(stmt: &Stmt<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match stmt {
        Stmt::Let(l) => walk_expr(l.init, on_path),
        Stmt::Expr(e) => walk_expr(&e.expr, on_path),
        Stmt::If(i) => walk_if(i, on_path),
        Stmt::Match(m) => {
            walk_expr(m.scrutinee, on_path);
            for arm in m.arms {
                for s in arm.body {
                    walk_stmt(s, on_path);
                }
            }
            if let Some(el) = m.else_body {
                for s in el {
                    walk_stmt(s, on_path);
                }
            }
        }
        Stmt::Loop(l) => {
            for s in l.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::Region(r) => {
            for s in r.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::For(f) => {
            walk_expr(f.iter, on_path);
            for s in f.body {
                walk_stmt(s, on_path);
            }
        }
        Stmt::Return(r) => {
            if let Some(e) = r.value {
                walk_expr(e, on_path);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        Stmt::Assign(a) => {
            walk_expr(a.target, on_path);
            walk_expr(a.value, on_path);
        }
    }
}

fn walk_if<'a>(i: &rynix_ast::IfStmt<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    for arm in i.arms {
        walk_expr(arm.cond, on_path);
        for s in arm.body {
            walk_stmt(s, on_path);
        }
    }
    if let Some(el) = i.else_body {
        for s in el {
            walk_stmt(s, on_path);
        }
    }
}

fn walk_expr<'a>(expr: &Expr<'a>, on_path: &mut dyn FnMut(&'a AstPath<'a>)) {
    match expr {
        Expr::Path(p) => on_path(p),
        Expr::Binary(b) => {
            walk_expr(b.lhs, on_path);
            walk_expr(b.rhs, on_path);
        }
        Expr::Unary(u) => walk_expr(u.operand, on_path),
        Expr::Call(c) => {
            walk_expr(c.callee, on_path);
            for a in c.args {
                walk_expr(a, on_path);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr(m.receiver, on_path);
            for a in m.args {
                walk_expr(a, on_path);
            }
        }
        Expr::Field(f) => walk_expr(f.base, on_path),
        Expr::StructLit(s) => {
            on_path(s.path);
            for init in s.fields {
                walk_expr(init.value, on_path);
            }
        }
        Expr::Index(i) => {
            walk_expr(i.base, on_path);
            walk_expr(i.index, on_path);
        }
        Expr::Cast(c) => walk_expr(c.expr, on_path),
        Expr::Array(a) => {
            for e in a.elems {
                walk_expr(e, on_path);
            }
        }
        Expr::Spawn(s) => walk_expr(s.callee, on_path),
        Expr::Literal(_) | Expr::Error(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rynix_ast::AstArena;
    use rynix_diag::VecSink;
    use rynix_span::{Interner, SourceMap};

    #[test]
    fn goto_def_resolves_fn_call() {
        let src = "def foo() -> i64\n  return 1\nend\ndef main() -> i64\n  return foo\nend\n";
        let mut sources = SourceMap::new();
        sources.add_owned("test.ryx", src.to_string());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let foo_ref = src.rfind("foo").unwrap() as u32 + file.start_pos();
        assert!(find_definition_span(module, &analysis, foo_ref).is_some());
    }

    #[test]
    fn lsp_workspace_def() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let app = root.join("testdata/ws_monorepo/app/main.ryx");
        let lib = root.join("testdata/ws_monorepo/lib/lib.ryx");
        let text = fs::read_to_string(&app).expect("app main");
        let mut sources = SourceMap::new();
        sources.add_owned(app.to_string_lossy().as_ref(), text.clone());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let needle = text.find("util_answer").expect("util_answer call") as u32 + file.start_pos();
        // Not defined in the open buffer alone.
        assert!(
            find_definition_span(module, &analysis, needle).is_none(),
            "expected no local def for util_answer"
        );
        let name = name_at_offset(module, &interner, needle).expect("name at offset");
        assert_eq!(name, "util_answer");
        let (path, span) = find_workspace_fn_def(&app, &name).expect("workspace def");
        let path_canon = fs::canonicalize(&path).unwrap();
        let lib_canon = fs::canonicalize(&lib).unwrap();
        assert_eq!(path_canon, lib_canon);
        let lib_text = fs::read_to_string(&lib).unwrap();
        let def_off = lib_text.find("util_answer").expect("def in lib") as u32;
        assert!(
            span.contains(def_off) || span.lo() == def_off,
            "span should cover util_answer in lib.ryx"
        );
    }

    #[test]
    fn hover_shows_type_for_literal() {
        let src = "def main() -> i64\n  return 42\nend\n";
        let mut sources = SourceMap::new();
        sources.add_owned("test.ryx", src.to_string());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let lit = src.find("42").unwrap() as u32 + file.start_pos();
        let hover = hover_at(module, &analysis, &interner, lit);
        assert_eq!(hover.as_deref(), Some("i64"));
    }

    #[test]
    fn completion_lists_fn_and_let() {
        let src = "def helper() -> i64\n  return 1\nend\ndef main() -> i64\n  let answer = 42\n  return answer\nend\n";
        let mut sources = SourceMap::new();
        sources.add_owned("test.ryx", src.to_string());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let at_return = src.rfind("answer").unwrap() as u32 + file.start_pos();
        let items = completion_items(&analysis, &interner, at_return, None);
        let labels: Vec<&str> = items
            .iter()
            .filter_map(|i| i["label"].as_str())
            .collect();
        assert!(
            labels.contains(&"helper"),
            "expected module fn helper: {labels:?}"
        );
        assert!(
            labels.contains(&"answer"),
            "expected let binding answer: {labels:?}"
        );
        assert!(
            labels.contains(&"main"),
            "expected module fn main: {labels:?}"
        );
        let prefixed = completion_items(&analysis, &interner, at_return, Some("hel"));
        let pref_labels: Vec<&str> = prefixed
            .iter()
            .filter_map(|i| i["label"].as_str())
            .collect();
        assert_eq!(pref_labels, vec!["helper"]);
    }

    #[test]
    fn rename_local_updates_def_and_refs() {
        let src = "def main() -> i64\n  let answer = 42\n  return answer\nend\n";
        let mut sources = SourceMap::new();
        sources.add_owned("test.ryx", src.to_string());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let use_off = src.rfind("answer").unwrap() as u32 + file.start_pos();
        let def_idx = def_index_at(module, &analysis, use_off).expect("def at use");
        let spans = reference_spans(module, &analysis, def_idx);
        assert!(
            spans.len() >= 2,
            "expected def + use spans, got {}",
            spans.len()
        );
        // Apply rename in source order to verify both sites.
        let mut renamed = src.to_string();
        let mut ordered = spans.clone();
        ordered.sort_by_key(|s| std::cmp::Reverse(s.lo()));
        for span in ordered {
            let lo = span.lo().saturating_sub(file.start_pos()) as usize;
            let hi = span.hi().saturating_sub(file.start_pos()) as usize;
            renamed.replace_range(lo..hi, "result");
        }
        assert!(
            renamed.contains("let result = 42"),
            "def not renamed: {renamed}"
        );
        assert!(
            renamed.contains("return result"),
            "use not renamed: {renamed}"
        );
        assert!(!renamed.contains("answer"), "old name remains: {renamed}");
    }

    #[test]
    fn references_lists_local_uses() {
        let src = "def main() -> i64\n  let answer = 42\n  return answer\nend\n";
        let mut sources = SourceMap::new();
        sources.add_owned("test.ryx", src.to_string());
        let file = sources.files().next().unwrap();
        let arena = AstArena::new();
        let mut interner = Interner::new();
        let mut sink = VecSink::new();
        let module = rynix_parser::parse(
            &arena,
            &mut interner,
            file.text(),
            file.start_pos(),
            &mut sink,
        );
        let analysis = analyze(module, &mut interner, &mut sink);
        let use_off = src.rfind("answer").unwrap() as u32 + file.start_pos();
        let def_idx = def_index_at(module, &analysis, use_off).expect("def at use");
        let spans = reference_spans(module, &analysis, def_idx);
        assert!(
            spans.len() >= 2,
            "expected def + use for references, got {}",
            spans.len()
        );
    }

    #[test]
    fn workspace_symbol_lists_fn() {
        let src = "def helper() -> i64\n  return 1\nend\ndef main() -> i64\n  return helper()\nend\n";
        let mut server = LanguageServer::new();
        server.documents.insert(
            "file:///test.ryx".into(),
            Document {
                path: PathBuf::from("test.ryx"),
                text: src.into(),
                version: 1,
            },
        );
        let req = LspRequest {
            id: Some(json!(1)),
            method: "workspace/symbol".into(),
            params: Some(json!({ "query": "hel" })),
        };
        let resp = server.workspace_symbol(&req);
        let arr = resp["result"].as_array().expect("result array");
        let names: Vec<&str> = arr.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names.iter().any(|n| *n == "helper"),
            "expected helper in workspace symbols: {names:?}"
        );
    }
}
