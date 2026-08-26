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
