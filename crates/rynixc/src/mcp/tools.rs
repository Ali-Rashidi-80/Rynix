//! MCP tool definitions, annotations, and dispatch.

use std::path::Path;

use crate::architecture::ArchitectureEngine;
use crate::attest::enrich_attest_json;
use crate::contract::ContractEngine;
use crate::dna::mine_dna;
use crate::lockfile::enrich_deps_json;
use crate::manifest::{find_manifest, load_manifest, resolve_deps};
use crate::mcp::transport::{rpc_error, tool_result_error};
use crate::scope::load_scope;
use crate::security::scan_source;

use rynix_ast::{AstArena, dump_module, format_module};
use rynix_codegen::emit_llvm;
use rynix_diag::{VecSink, render_json};
use rynix_rir::{analyze_escape, explain_alloc_json, inject_regions, lower_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use serde_json::{Value, json};

fn path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "source": { "type": "string" },
            "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
        }
    })
}

pub(crate) fn tool_defs() -> Value {
    json!([
        {
            "name": "diagnostics",
            "description": "Alias of rynix_check — structured diagnostics. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "rynix_check",
            "description": "Lex+parse+sema; return diagnostics. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "rynix_format",
            "description": "Canonical-format Rynix. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "rynix_explain_alloc",
            "description": "Escape/placement report. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "compile",
            "description": "Lower+escape+opt and emit textual LLVM IR (.ll). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "ast_query",
            "description": "Parse and return the s-expression AST dump. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "apply_fix",
            "description": "Apply the first suggested Fix from diagnostics, if any. Prefer path (reads disk); returns fixed text (does not write).",
            "inputSchema": path_schema(),
            "annotations": { "readOnlyHint": false, "destructiveHint": true, "openWorldHint": false }
        },
        {
            "name": "rynix_graph",
            "description": "Emit rynix.graph.v1 (functions + static call edges). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": path_schema(),
            "outputSchema": { "type": "object", "properties": { "schema": { "const": "rynix.graph.v1" } } },
            "annotations": { "readOnlyHint": true, "destructiveHint": false, "openWorldHint": false }
        },
        {
            "name": "rynix_slice",
            "description": "Compact interface outline (rynix.slice.v1) — same as `rynixc slice`. Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "rynix_impact",
            "description": "Blast-radius callers/callees (rynix.impact.v1). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" },
                    "fn": { "type": "string" }
                }
            }
        },
        {
            "name": "rynix_eval",
            "description": "Micro-evaluate a Rynix expression via RIR interpreter",
            "inputSchema": {
                "type": "object",
                "properties": { "expr": { "type": "string" } },
                "required": ["expr"]
            }
        },
        {
            "name": "rynix_arch",
            "description": "Run Architecture.toml layer check (rynix.arch.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "root": { "type": "string", "description": "Project root (default .)" },
                    "config": { "type": "string", "description": "Path to Architecture.toml" }
                }
            }
        },
        {
            "name": "rynix_verify",
            "description": "Verify agent contract evidence (rynix.verify.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "contract": { "type": "string", "description": "Path to contract TOML" },
                    "root": { "type": "string" },
                    "run": { "type": "boolean", "description": "Run cargo_test evidence" }
                },
                "required": ["contract"]
            }
        },
        {
            "name": "rynix_precheck",
            "description": "Blast-radius + write gate (rynix.precheck.v1). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" },
                    "fn": { "type": "string" },
                    "allow_write": { "type": "boolean" }
                }
            }
        },
        {
            "name": "rynix_context",
            "description": "Budgeted interface outline (rynix.context.v1). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" },
                    "budget": { "type": "integer", "minimum": 1 }
                }
            }
        },
        {
            "name": "rynix_security",
            "description": "Pattern CWE-798-class scan (rynix.security.v1). Prefer path (reads disk); source is optional inline body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string", "description": "Filesystem .ryx path (path-first)" }
                }
            }
        },
        {
            "name": "rynix_scope",
            "description": "Agent permission profile (rynix.scope.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "config": { "type": "string" }
                }
            }
        },
        {
            "name": "rynix_deps",
            "description": "Resolve local path/registry deps from rynix.toml (rynix.deps.v1; local index only)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File or directory to start search" }
                }
            }
        },
        {
            "name": "rynix_dna",
            "description": "Heuristic project conventions (rynix.dna.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root (default .)" },
                    "prompt": { "type": "boolean", "description": "Prefer agent prompt text" }
                }
            }
        }
    ])
}

pub(crate) fn tools_call(params: &Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| rpc_error(-32602, "missing tool name"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    if name == "rynix_eval" {
        let expr = args
            .get("expr")
            .and_then(|e| e.as_str())
            .ok_or_else(|| rpc_error(-32602, "missing expr"))?;
        let result = crate::agent_lib::eval_snippet(expr).map_err(|e| rpc_error(-32000, e))?;
        let out = json!({ "schema": "rynix.eval.v1", "result": result });
        return Ok(json!({ "content": [{ "type": "text", "text": out.to_string() }] }));
    }

    if name == "rynix_arch" {
        let root = args.get("root").and_then(|r| r.as_str()).unwrap_or(".");
        let config_path = args.get("config").and_then(|c| c.as_str()).map(Path::new);
        let config =
            ArchitectureEngine::load_config(config_path).map_err(|e| rpc_error(-32000, e))?;
        let report = ArchitectureEngine::check_project(&config, Path::new(root));
        return Ok(json!({
            "content": [{ "type": "text", "text": report.to_json().to_string() }]
        }));
    }

    if name == "rynix_verify" {
        let contract_path = args
            .get("contract")
            .and_then(|c| c.as_str())
            .ok_or_else(|| rpc_error(-32602, "missing contract"))?;
        let root = args.get("root").and_then(|r| r.as_str()).unwrap_or(".");
        let run = args.get("run").and_then(|r| r.as_bool()).unwrap_or(false);
        let contract =
            ContractEngine::load(Path::new(contract_path)).map_err(|e| rpc_error(-32000, e))?;
        let report = ContractEngine::verify(&contract, Path::new(root), run);
        return Ok(json!({
            "content": [{ "type": "text", "text": report.to_json().to_string() }]
        }));
    }

    if name == "rynix_scope" {
        let config = args.get("config").and_then(|c| c.as_str()).map(Path::new);
        let report = load_scope(config);
        return Ok(json!({
            "content": [{ "type": "text", "text": report.to_json().to_string() }]
        }));
    }

    if name == "rynix_deps" {
        let start = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
        let Some(m_path) = find_manifest(Path::new(start)) else {
            return Err(rpc_error(-32000, "no rynix.toml found"));
        };
        let manifest = load_manifest(&m_path).map_err(|e| rpc_error(-32000, e))?;
        let report = resolve_deps(&manifest);
        let body = enrich_attest_json(&report, enrich_deps_json(&report, report.to_json()));
        return Ok(json!({
            "content": [{ "type": "text", "text": body.to_string() }]
        }));
    }

    if name == "rynix_dna" {
        let start = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
        let report = mine_dna(Path::new(start));
        let prompt = args
            .get("prompt")
            .and_then(|p| p.as_bool())
            .unwrap_or(false);
        let text = if prompt {
            report.to_prompt()
        } else {
            report.to_json().to_string()
        };
        return Ok(json!({
            "content": [{ "type": "text", "text": text }]
        }));
    }

    // Path-first graph: read disk when `path` is set; inline `source` optional.
    if name == "rynix_graph" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let arena = AstArena::new();
        let (label, mut parsed) = resolve_path_or_source(path_arg, source_arg, &arena)?;
        if parsed.sink.error_count() > 0 {
            return Err(rpc_error(-32000, "parse/sema errors"));
        }
        let g = if let (Some(path), None) = (path_arg, source_arg) {
            crate::agent_lib::graph_json(Path::new(path), &mut parsed)
        } else {
            crate::agent_lib::graph_json_text(&label, &mut parsed)
        };
        return Ok(json!({ "content": [{ "type": "text", "text": g.to_string() }] }));
    }

    // Path-first slice (parity with `rynixc slice`).
    if name == "rynix_slice" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let arena = AstArena::new();
        let (label, parsed) = resolve_path_or_source(path_arg, source_arg, &arena)?;
        if parsed.sink.error_count() > 0 {
            return Err(rpc_error(-32000, "parse/sema errors"));
        }
        let lines = crate::agent_lib::slice_lines(&parsed);
        let report = json!({
            "schema": "rynix.slice.v1",
            "path": label,
            "lines": lines,
        });
        return Ok(json!({ "content": [{ "type": "text", "text": report.to_string() }] }));
    }

    // Path-first impact / precheck (same fail-closed disk read as graph).
    if name == "rynix_impact" || name == "rynix_precheck" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let arena = AstArena::new();
        let (label, mut parsed) = resolve_path_or_source(path_arg, source_arg, &arena)?;
        if parsed.sink.error_count() > 0 {
            return Err(rpc_error(-32000, "parse/sema errors"));
        }
        let target = args.get("fn").and_then(|f| f.as_str());
        let impact = crate::agent_lib::impact_json(&label, &mut parsed, target)
            .map_err(|e| rpc_error(-32000, e))?;
        if name == "rynix_impact" {
            return Ok(json!({ "content": [{ "type": "text", "text": impact.to_string() }] }));
        }
        let allow_write = args
            .get("allow_write")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        let report = json!({
            "schema": "rynix.precheck.v1",
            "path": label,
            "write_allowed": allow_write,
            "fn": target,
            "impact": impact,
        });
        return Ok(json!({ "content": [{ "type": "text", "text": report.to_string() }] }));
    }

    // Path-first check / apply_fix / context / security.
    if name == "rynix_check" || name == "diagnostics" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let (label, text) = resolve_label_and_text(path_arg, source_arg)?;
        let out = check_source(&label, &text);
        return Ok(json!({ "content": [{ "type": "text", "text": out }] }));
    }
    if name == "apply_fix" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let (_label, text) = resolve_label_and_text(path_arg, source_arg)?;
        let fixed = apply_fix_source(&text);
        return Ok(json!({ "content": [{ "type": "text", "text": fixed }] }));
    }
    if name == "rynix_context" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let arena = AstArena::new();
        let (label, parsed) = resolve_path_or_source(path_arg, source_arg, &arena)?;
        if parsed.sink.error_count() > 0 {
            return Err(rpc_error(-32000, "parse/sema errors"));
        }
        let budget = args.get("budget").and_then(|b| b.as_u64()).unwrap_or(2000) as usize;
        let budget = budget.max(1);
        let all = crate::agent_lib::slice_lines(&parsed);
        let mut lines = Vec::new();
        let mut used = 0usize;
        let mut truncated = false;
        for line in all {
            let add = if lines.is_empty() {
                line.len()
            } else {
                line.len() + 1
            };
            if used + add > budget {
                truncated = true;
                break;
            }
            used += add;
            lines.push(line);
        }
        let report = json!({
            "schema": "rynix.context.v1",
            "path": label,
            "budget": budget,
            "chars_used": used,
            "truncated": truncated,
            "lines": lines,
        });
        return Ok(json!({ "content": [{ "type": "text", "text": report.to_string() }] }));
    }
    if name == "rynix_security" {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let (label, text) = resolve_label_and_text(path_arg, source_arg)?;
        let report = scan_source(&label, &text);
        return Ok(
            json!({ "content": [{ "type": "text", "text": report.to_json().to_string() }] }),
        );
    }

    // Path-first format / explain / compile / ast_query (Phase 22).
    if name == "rynix_format"
        || name == "rynix_explain_alloc"
        || name == "compile"
        || name == "ast_query"
    {
        let path_arg = args.get("path").and_then(|p| p.as_str());
        let source_arg = args.get("source").and_then(|s| s.as_str());
        let (_label, text) = resolve_label_and_text(path_arg, source_arg)?;
        let out = match name {
            "rynix_format" => format_source(&text)?,
            "rynix_explain_alloc" => explain_source(&text)?,
            "compile" => compile_source(&text)?,
            "ast_query" => ast_source(&text)?,
            _ => unreachable!(),
        };
        return Ok(json!({ "content": [{ "type": "text", "text": out }] }));
    }

    Ok(tool_result_error(format!("unknown tool: {name}")))
}

fn check_source(path: &str, source: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);
    let mut sm = SourceMap::new();
    sm.add_owned(path, source.to_string());
    let mut out = String::new();
    for d in &sink.diags {
        out.push_str(&render_json(d, &sm));
        out.push('\n');
    }
    if out.is_empty() {
        out.push_str("{\"ok\":true}\n");
    }
    out
}

fn format_source(source: &str) -> Result<String, Value> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    if sink.error_count() > 0 {
        return Err(rpc_error(-32000, "parse errors; cannot format"));
    }
    Ok(format_module(module, &interner, source, 0))
}

fn explain_source(source: &str) -> Result<String, Value> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    let analysis = analyze(module, &mut interner, &mut sink);
    if sink.error_count() > 0 {
        return Err(rpc_error(-32000, "sema/parse errors"));
    }
    let rir = lower_module(module, &analysis, &mut interner, source, 0);
    let report = analyze_escape(&rir, &interner);
    Ok(explain_alloc_json(&rir, &report, &interner))
}

fn compile_source(source: &str) -> Result<String, Value> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    let analysis = analyze(module, &mut interner, &mut sink);
    if sink.error_count() > 0 {
        return Err(rpc_error(-32000, "sema/parse errors"));
    }
    let mut rir = lower_module(module, &analysis, &mut interner, source, 0);
    let report = analyze_escape(&rir, &interner);
    inject_regions(&mut rir, &report);
    let _ = run_pipeline(&mut rir);
    Ok(emit_llvm(&rir, &interner, Some(&report)))
}

fn ast_source(source: &str) -> Result<String, Value> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    if sink.error_count() > 0 {
        return Err(rpc_error(-32000, "parse errors"));
    }
    Ok(dump_module(module, &interner))
}

fn apply_fix_source(source: &str) -> String {
    crate::fix::apply_first_fix(source)
}

/// Path-first source resolution: disk read when only `path`; fail-closed on missing file.
fn resolve_path_or_source<'a>(
    path_arg: Option<&str>,
    source_arg: Option<&str>,
    arena: &'a AstArena,
) -> Result<(String, crate::agent_lib::Parsed<'a>), Value> {
    match (path_arg, source_arg) {
        (Some(path), None) => {
            let parsed = crate::agent_lib::parse_file(Path::new(path), arena)
                .map_err(|e| rpc_error(-32000, e))?;
            Ok((path.to_string(), parsed))
        }
        (Some(path), Some(source)) => {
            let parsed = crate::agent_lib::parse_text(path, source, arena);
            Ok((path.to_string(), parsed))
        }
        (None, Some(source)) => {
            let parsed = crate::agent_lib::parse_text("mcp.ryx", source, arena);
            Ok(("mcp.ryx".to_string(), parsed))
        }
        (None, None) => Err(rpc_error(-32602, "missing path or source")),
    }
}

/// Path-first text resolution (no parse): for check / security / apply_fix.
fn resolve_label_and_text(
    path_arg: Option<&str>,
    source_arg: Option<&str>,
) -> Result<(String, String), Value> {
    match (path_arg, source_arg) {
        (Some(path), None) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| rpc_error(-32000, format!("failed to read {path}: {e}")))?;
            Ok((path.to_string(), text))
        }
        (Some(path), Some(source)) => Ok((path.to_string(), source.to_string())),
        (None, Some(source)) => Ok(("mcp.ryx".to_string(), source.to_string())),
        (None, None) => Err(rpc_error(-32602, "missing path or source")),
    }
}
