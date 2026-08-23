//! Shared helpers for graph / impact / eval / MCP agent tools.

use rynix_ast::{AstArena, Item, Stmt};
use rynix_diag::VecSink;
use rynix_rir::{interpret_module, lower_module, module_call_graph, run_pipeline, InterpValue, Module};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use serde_json::{json, Value};

pub struct Parsed<'a> {
    pub sources: SourceMap,
    pub sink: VecSink,
    pub interner: Interner,
    pub module: &'a rynix_ast::Module<'a>,
    pub src: String,
    pub base: u32,
}

pub fn parse_file<'a>(path: &std::path::Path, arena: &'a AstArena) -> Result<Parsed<'a>, String> {
    let mut sources = SourceMap::new();
    let file_id = sources
        .load_file(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file = sources.file(file_id);
    let src = file.text().to_string();
    let base = file.start_pos();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(arena, &mut interner, &src, base, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);
    Ok(Parsed {
        sources,
        sink,
        interner,
        module,
        src,
        base,
    })
}

pub fn parse_text<'a>(label: &str, source: &str, arena: &'a AstArena) -> Parsed<'a> {
    let mut sources = SourceMap::new();
    sources.add_owned(label, source.to_string());
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(arena, &mut interner, source, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);
    Parsed {
        sources,
        sink,
        interner,
        module,
        src: source.to_string(),
        base: 0,
    }
}

fn lower_module_checked(parsed: &mut Parsed<'_>) -> Result<Module, String> {
    if parsed.sink.error_count() > 0 {
        return Err(format!("{} sema/parse errors", parsed.sink.error_count()));
    }
    let analysis = analyze(parsed.module, &mut parsed.interner, &mut parsed.sink);
    if parsed.sink.error_count() > 0 {
        return Err("sema errors".into());
    }
    let mut rir = lower_module(
        parsed.module,
        &analysis,
        &mut parsed.interner,
        &parsed.src,
        parsed.base,
    );
    let errs = run_pipeline(&mut rir);
    if !errs.is_empty() {
        return Err(format!("RIR verify: {errs:?}"));
    }
    Ok(rir)
}

fn function_summaries(parsed: &Parsed<'_>) -> Vec<Value> {
    parsed
        .module
        .items
        .iter()
        .filter_map(|item| {
            let Item::Fn(f) = item else {
                return None;
            };
            let name = parsed.interner.resolve(f.name.name);
            let params: Vec<_> = f
                .params
                .iter()
                .map(|p| parsed.interner.resolve(p.name.name).to_string())
                .collect();
            let ret = match f.ret {
                Some(rynix_ast::Type::Path(p)) if p.segments.len() == 1 => {
                    parsed.interner.resolve(p.segments[0].name).to_string()
                }
                Some(_) => "ty".into(),
                None => "unit".into(),
            };
            Some(json!({
                "name": name,
                "params": params,
                "ret": ret,
                "span": { "lo": f.span.lo(), "hi": f.span.hi() },
            }))
        })
        .collect()
}

fn call_edges(parsed: &mut Parsed<'_>) -> Vec<Value> {
    let Ok(rir) = lower_module_checked(parsed) else {
        return Vec::new();
    };
    let names: Vec<String> = rir
        .func_names
        .iter()
        .map(|s| parsed.interner.resolve(*s).to_string())
        .collect();
    let g = module_call_graph(&rir);
    let mut edges = Vec::new();
    for (from, callees) in g.iter().enumerate() {
        let from_name = names.get(from).cloned().unwrap_or_else(|| format!("fn{from}"));
        for &to in callees {
            let to_name = names.get(to).cloned().unwrap_or_else(|| format!("fn{to}"));
            edges.push(json!({ "from": from_name, "to": to_name }));
        }
    }
    edges
}

/// Compact interface outline used by `slice` and `context`.
pub fn slice_lines(parsed: &Parsed<'_>) -> Vec<String> {
    let mut lines = Vec::new();
    for item in parsed.module.items {
        match item {
            Item::Fn(f) => {
                let name = parsed.interner.resolve(f.name.name);
                let params: Vec<_> = f
                    .params
                    .iter()
                    .map(|p| parsed.interner.resolve(p.name.name).to_string())
                    .collect();
                let body_hint = f.body.first().map_or("empty", |s| match s {
                    Stmt::Return(_) => "return",
                    Stmt::Let(_) => "let",
                    Stmt::If(_) => "if",
                    Stmt::Match(_) => "match",
                    Stmt::Loop(_) | Stmt::For(_) => "loop",
                    Stmt::Region(_) => "region",
                    _ => "stmt",
                });
                lines.push(format!(
                    "def {name}({})  # body:{body_hint}",
                    params.join(", ")
                ));
            }
            Item::Struct(s) => {
                lines.push(format!(
                    "struct {}  # fields:{}",
                    parsed.interner.resolve(s.name.name),
                    s.fields.len()
                ));
            }
            Item::Enum(e) => {
                lines.push(format!(
                    "enum {}  # variants:{}",
                    parsed.interner.resolve(e.name.name),
                    e.variants.len()
                ));
            }
            _ => {}
        }
    }
    lines
}

pub fn graph_json(path: &std::path::Path, parsed: &mut Parsed<'_>) -> Value {
    json!({
        "schema": "rynix.graph.v1",
        "path": path.display().to_string(),
        "functions": function_summaries(parsed),
        "edges": call_edges(parsed),
    })
}

pub fn graph_json_text(label: &str, parsed: &mut Parsed<'_>) -> Value {
    json!({
        "schema": "rynix.graph.v1",
        "path": label,
        "functions": function_summaries(parsed),
        "edges": call_edges(parsed),
    })
}

pub fn impact_json(
    path: &str,
    parsed: &mut Parsed<'_>,
    target: Option<&str>,
) -> Result<Value, String> {
    let rir = lower_module_checked(parsed)?;
    let names: Vec<String> = rir
        .func_names
        .iter()
        .map(|s| parsed.interner.resolve(*s).to_string())
        .collect();
    let g = module_call_graph(&rir);
    let mut caller_idx: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (from, outs) in g.iter().enumerate() {
        for &to in outs {
            if to < caller_idx.len() {
                caller_idx[to].push(from);
            }
        }
    }
    let indices: Vec<usize> = if let Some(t) = target {
        let idx = names
            .iter()
            .position(|n| n == t)
            .ok_or_else(|| format!("unknown function `{t}`"))?;
        vec![idx]
    } else {
        (0..names.len()).collect()
    };
    let mut nodes = Vec::new();
    for i in indices {
        let outs: Vec<_> = g
            .get(i)
            .map(|v| {
                v.iter()
                    .filter_map(|&c| names.get(c).cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let ins: Vec<_> = caller_idx
            .get(i)
            .map(|v| {
                v.iter()
                    .filter_map(|&c| names.get(c).cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        nodes.push(json!({
            "fn": names.get(i).cloned().unwrap_or_else(|| format!("fn{i}")),
            "callees": outs,
            "callers": ins,
        }));
    }
    Ok(json!({
        "schema": "rynix.impact.v1",
        "path": path,
        "target": target,
        "nodes": nodes,
    }))
}

pub fn wrap_eval_snippet(snippet: &str) -> String {
    let s = snippet.trim();
    if s.contains("def main") {
        return format!("{s}\n");
    }
    let needs_body = s.contains('\n')
        || s.starts_with("let ")
        || s.starts_with("if ")
        || s.starts_with("match ")
        || s.starts_with("loop")
        || s.starts_with("return ");
    if needs_body {
        if s.contains("return ") {
            format!("def main() -> i64\n{s}\nend\n")
        } else {
            format!("def main() -> i64\n{s}\n  return 0\nend\n")
        }
    } else {
        format!("def main() -> i64\n  return {s}\nend\n")
    }
}

pub fn eval_snippet(snippet: &str) -> Result<Value, String> {
    let wrapped = wrap_eval_snippet(snippet);
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, &wrapped, 0, &mut sink);
    if sink.error_count() > 0 {
        return Err(format!("parse: {} errors", sink.error_count()));
    }
    let analysis = analyze(module, &mut interner, &mut sink);
    if sink.error_count() > 0 {
        return Err(format!("sema: {} errors", sink.error_count()));
    }
    let mut rir = lower_module(module, &analysis, &mut interner, &wrapped, 0);
    let errs = run_pipeline(&mut rir);
    if !errs.is_empty() {
        return Err(format!("RIR: {errs:?}"));
    }
    let v = interpret_module(&rir, &interner).map_err(|e| format!("{e:?}"))?;
    Ok(interp_to_json(&v))
}

fn interp_to_json(v: &InterpValue) -> Value {
    match v {
        InterpValue::I64(n) => json!({ "type": "i64", "value": n }),
        InterpValue::Bool(b) => json!({ "type": "bool", "value": b }),
        InterpValue::F64(n) => json!({ "type": "f64", "value": n }),
        InterpValue::Str(s) => json!({ "type": "str", "value": s }),
        InterpValue::Unit => json!({ "type": "unit" }),
        other => json!({ "type": "other", "debug": format!("{other:?}") }),
    }
}
