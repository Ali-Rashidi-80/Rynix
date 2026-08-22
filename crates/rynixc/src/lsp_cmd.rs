//! Minimal LSP server on stdio: full-sync documents, diagnostics, go-to-definition.

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
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

use rynix_ast::{Expr, Item, Module, Path as AstPath, Stmt};
use rynix_ast::{AstArena, Ident};
use rynix_diag::VecSink;
use rynix_sema::{analyze, Analysis};
use rynix_span::{SourceMap, Span};
use serde_json::{json, Value};

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
                        "hoverProvider": true
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

        let def_span = find_definition_span(module, &analysis, offset);
        let result = def_span.map(|span| {
            let (_f, start) = sources.line_col(span.lo());
            let (_, end) = sources.line_col(span.hi());
            json!({
                "uri": uri,
                "range": {
                    "start": { "line": start.line.saturating_sub(1), "character": start.col.saturating_sub(1) },
                    "end": { "line": end.line.saturating_sub(1), "character": end.col.saturating_sub(1) }
                }
            })
        });

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
}
