use std::fs;
use std::path::PathBuf;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use serde_json::json;

use super::*;

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

#[test]
fn document_symbol_lists_fn() {
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
        method: "textDocument/documentSymbol".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" }
        })),
    };
    let resp = server.document_symbol(&req);
    let arr = resp["result"].as_array().expect("result array");
    let names: Vec<&str> = arr.iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(
        names.iter().any(|n| *n == "helper"),
        "expected helper in document symbols: {names:?}"
    );
    assert!(
        names.iter().any(|n| *n == "main"),
        "expected main in document symbols: {names:?}"
    );
}

#[test]
fn lsp_formatting_applies_fmt() {
    // Messy spacing — formatter should canonicalize like `rynixc fmt`.
    let src = "def   main()->i64\nreturn 1\nend\n";
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
        method: "textDocument/formatting".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" },
            "options": { "tabSize": 2, "insertSpaces": true }
        })),
    };
    let resp = server.formatting(&req);
    let edits = resp["result"].as_array().expect("result array");
    assert_eq!(edits.len(), 1, "expected one full-document TextEdit: {edits:?}");
    let new_text = edits[0]["newText"].as_str().expect("newText");
    assert!(
        new_text.contains("def main() -> i64"),
        "expected fmt spacing around -> : {new_text:?}"
    );
    assert!(
        new_text.contains("  return 1"),
        "expected indented return: {new_text:?}"
    );
    assert_ne!(new_text, src, "formatting should change messy input");
}

#[test]
fn lsp_code_action_smoke() {
    // Missing `end` — parser attaches insert-`end` Fix (same pipeline as apply_fix).
    let src = "def main() -> i64\n  return 1\n";
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
        method: "textDocument/codeAction".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 3 }
            },
            "context": { "diagnostics": [] }
        })),
    };
    let resp = server.code_action(&req);
    let actions = resp["result"].as_array().expect("result array");
    assert!(
        !actions.is_empty(),
        "expected at least one codeAction for missing end: {resp}"
    );
    let titles: Vec<&str> = actions
        .iter()
        .filter_map(|a| a["title"].as_str())
        .collect();
    assert!(
        titles.iter().any(|t| t.contains("end")),
        "expected insert end quickfix, got {titles:?}"
    );
    let edit = &actions[0]["edit"]["changes"]["file:///test.ryx"];
    let edits = edit.as_array().expect("TextEdit array");
    assert!(!edits.is_empty(), "expected WorkspaceEdit TextEdits");
    assert!(
        edits.iter().any(|e| e["newText"].as_str() == Some("end\n")),
        "expected end\\n replacement: {edits:?}"
    );

    // Clean source → no actions.
    let clean = "def main() -> i64\n  return 1\nend\n";
    server.documents.insert(
        "file:///clean.ryx".into(),
        Document {
            path: PathBuf::from("clean.ryx"),
            text: clean.into(),
            version: 1,
        },
    );
    let req2 = LspRequest {
        id: Some(json!(2)),
        method: "textDocument/codeAction".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///clean.ryx" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 2, "character": 0 }
            },
            "context": { "diagnostics": [] }
        })),
    };
    let resp2 = server.code_action(&req2);
    let empty = resp2["result"].as_array().expect("result");
    assert!(empty.is_empty(), "clean file should have no codeActions: {resp2}");
}

#[test]
fn lsp_prepare_rename_smoke() {
    let src = "def main() -> i64\n  let answer = 42\n  return answer\nend\n";
    let mut server = LanguageServer::new();
    server.documents.insert(
        "file:///test.ryx".into(),
        Document {
            path: PathBuf::from("test.ryx"),
            text: src.into(),
            version: 1,
        },
    );
    let use_line = src.lines().position(|l| l.contains("return answer")).unwrap();
    let use_col = src.lines().nth(use_line).unwrap().find("answer").unwrap();
    let req = LspRequest {
        id: Some(json!(1)),
        method: "textDocument/prepareRename".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" },
            "position": { "line": use_line, "character": use_col }
        })),
    };
    let resp = server.prepare_rename(&req);
    let range = resp["result"]["range"].as_object().expect("prepareRename range");
    assert_eq!(range["start"]["line"].as_u64(), Some(use_line as u64));
    assert!(
        range["start"]["character"].as_u64().unwrap() <= use_col as u64,
        "range should start at answer: {range:?}"
    );
}

#[test]
fn lsp_document_highlight_smoke() {
    let src = "def main() -> i64\n  let answer = 42\n  return answer\nend\n";
    let mut server = LanguageServer::new();
    server.documents.insert(
        "file:///test.ryx".into(),
        Document {
            path: PathBuf::from("test.ryx"),
            text: src.into(),
            version: 1,
        },
    );
    let use_line = src.lines().position(|l| l.contains("return answer")).unwrap();
    let use_col = src.lines().nth(use_line).unwrap().find("answer").unwrap();
    let req = LspRequest {
        id: Some(json!(1)),
        method: "textDocument/documentHighlight".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" },
            "position": { "line": use_line, "character": use_col }
        })),
    };
    let resp = server.document_highlight(&req);
    let highlights = resp["result"].as_array().expect("highlights");
    assert!(
        highlights.len() >= 2,
        "expected def + use highlights: {highlights:?}"
    );
}

#[test]
fn lsp_inlay_hint_smoke() {
    let src = "def main() -> i64\n  let answer = 42\n  return answer\nend\n";
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
        method: "textDocument/inlayHint".into(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.ryx" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 10, "character": 0 }
            }
        })),
    };
    let resp = server.inlay_hint(&req);
    let hints = resp["result"].as_array().expect("inlay hints");
    assert!(
        hints.iter().any(|h| h["label"].as_str().is_some_and(|l| l.contains("i64"))),
        "expected i64 inlay hint for answer: {hints:?}"
    );
}
