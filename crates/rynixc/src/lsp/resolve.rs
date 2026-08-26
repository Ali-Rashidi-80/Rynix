//! Definition, hover, completion, rename, and workspace lookup helpers.

use std::fs;
use std::path::{Path, PathBuf};

use rynix_ast::{Item, Module, Path as AstPath};
use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::{Analysis, DefKind};
use rynix_span::Span;
use serde_json::{json, Value};

use crate::lsp::walk::{find_expr_at, find_ident_at, find_path_at, walk_item};
use crate::manifest::{find_workspace_root, load_manifest};

pub(crate) fn name_at_offset(
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
pub(crate) fn workspace_member_sources(from: &Path) -> Vec<PathBuf> {
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
pub(crate) fn find_fn_def_span_in_file(path: &Path, name: &str) -> Option<Span> {
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
pub(crate) fn find_workspace_fn_def(doc_path: &Path, name: &str) -> Option<(PathBuf, Span)> {
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

pub(crate) fn find_definition_span(module: &Module<'_>, analysis: &Analysis, offset: u32) -> Option<Span> {
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

pub(crate) fn hover_at(
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
pub(crate) fn completion_prefix(text: &str, file_start: u32, offset: u32) -> Option<String> {
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

pub(crate) fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// Local/module function and let (and param) bindings visible near `offset`.
pub(crate) fn completion_items(
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

pub(crate) fn def_index_at(module: &Module<'_>, analysis: &Analysis, offset: u32) -> Option<usize> {
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

pub(crate) fn renameable_def(def: &DefKind) -> bool {
    matches!(
        def,
        DefKind::Fn { .. } | DefKind::Local { .. } | DefKind::Param { .. }
    )
}

pub(crate) fn reference_spans(module: &Module<'_>, analysis: &Analysis, def_idx: usize) -> Vec<Span> {
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

