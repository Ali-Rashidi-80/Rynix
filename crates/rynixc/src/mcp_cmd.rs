//! `rynixc mcp-serve` — JSON-RPC 2.0 over stdio (Content-Length framing).
//!
//! Tools: `diagnostics`/`rynix_check`, `rynix_format`, `rynix_explain_alloc`,
//! `compile`, `ast_query`, `apply_fix`, `rynix_graph`, `rynix_impact`, `rynix_eval`,
//! `rynix_arch`, `rynix_verify`, `rynix_precheck`, `rynix_context`, `rynix_security`,
//! `rynix_scope`, `rynix_deps`, `rynix_dna`.

#![allow(clippy::too_many_lines)]

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

use crate::architecture::ArchitectureEngine;
use crate::contract::ContractEngine;
use crate::dna::mine_dna;
use crate::lockfile::enrich_deps_json;
use crate::manifest::{find_manifest, load_manifest, resolve_deps};
use crate::scope::load_scope;
use crate::security::scan_source;

use rynix_ast::{dump_module, format_module, AstArena};
use rynix_codegen::emit_llvm;
use rynix_diag::{render_json, VecSink};
use rynix_rir::{analyze_escape, explain_alloc_json, inject_regions, lower_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use serde_json::{json, Value};

pub fn run() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        let msg = match read_message(&mut reader) {
            Ok(Some(v)) => v,
            Ok(None) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("mcp: read error: {e}");
                return ExitCode::from(1);
            }
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        if method == "notifications/initialized" || method.starts_with("notifications/") {
            continue;
        }

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "rynixc", "version": env!("CARGO_PKG_VERSION") }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_defs() })),
            "tools/call" => tools_call(&params),
            "shutdown" => {
                write_result(&mut stdout, id, Ok(Value::Null));
                return ExitCode::SUCCESS;
            }
            "" if msg.get("result").is_some() => continue,
            other => Err(rpc_error(-32601, format!("method not found: {other}"))),
        };

        if id.is_some() {
            write_result(&mut stdout, id, result);
        }
    }
}

fn tool_defs() -> Value {
    json!([
        {
            "name": "diagnostics",
            "description": "Alias of rynix_check — structured diagnostics for a source string",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_check",
            "description": "Lex+parse+sema a Rynix source string; return diagnostics",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_format",
            "description": "Canonical-format a Rynix source string",
            "inputSchema": {
                "type": "object",
                "properties": { "source": { "type": "string" } },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_explain_alloc",
            "description": "Escape/placement report for a clean Rynix source string",
            "inputSchema": {
                "type": "object",
                "properties": { "source": { "type": "string" } },
                "required": ["source"]
            }
        },
        {
            "name": "compile",
            "description": "Lower+escape+opt and emit textual LLVM IR (.ll)",
            "inputSchema": {
                "type": "object",
                "properties": { "source": { "type": "string" } },
                "required": ["source"]
            }
        },
        {
            "name": "ast_query",
            "description": "Parse and return the s-expression AST dump",
            "inputSchema": {
                "type": "object",
                "properties": { "source": { "type": "string" } },
                "required": ["source"]
            }
        },
        {
            "name": "apply_fix",
            "description": "Apply the first suggested Fix from diagnostics, if any",
            "inputSchema": {
                "type": "object",
                "properties": { "source": { "type": "string" } },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_graph",
            "description": "Emit rynix.graph.v1 (functions + static call edges)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_impact",
            "description": "Blast-radius callers/callees (rynix.impact.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" },
                    "fn": { "type": "string" }
                },
                "required": ["source"]
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
            "description": "Blast-radius + write gate (rynix.precheck.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" },
                    "fn": { "type": "string" },
                    "allow_write": { "type": "boolean" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_context",
            "description": "Budgeted interface outline (rynix.context.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" },
                    "budget": { "type": "integer", "minimum": 1 }
                },
                "required": ["source"]
            }
        },
        {
            "name": "rynix_security",
            "description": "Pattern CWE-798-class scan (rynix.security.v1)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["source"]
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

fn tools_call(params: &Value) -> Result<Value, Value> {
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
        let result = crate::agent_lib::eval_snippet(expr)
            .map_err(|e| rpc_error(-32000, e))?;
        let out = json!({ "schema": "rynix.eval.v1", "result": result });
        return Ok(json!({ "content": [{ "type": "text", "text": out.to_string() }] }));
    }

    if name == "rynix_arch" {
        let root = args
            .get("root")
            .and_then(|r| r.as_str())
            .unwrap_or(".");
        let config_path = args.get("config").and_then(|c| c.as_str()).map(Path::new);
        let config = ArchitectureEngine::load_config(config_path)
            .map_err(|e| rpc_error(-32000, e))?;
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
        let run = args
            .get("run")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);
        let contract = ContractEngine::load(Path::new(contract_path))
            .map_err(|e| rpc_error(-32000, e))?;
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
        let start = args
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");
        let Some(m_path) = find_manifest(Path::new(start)) else {
            return Err(rpc_error(-32000, "no rynix.toml found"));
        };
        let manifest = load_manifest(&m_path).map_err(|e| rpc_error(-32000, e))?;
        let report = resolve_deps(&manifest);
        let body = enrich_deps_json(&report, report.to_json());
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

    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .ok_or_else(|| rpc_error(-32602, "missing source"))?;

    match name {
        "rynix_check" | "diagnostics" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("mcp.ryx");
            let text = check_source(path, source);
            Ok(json!({ "content": [{ "type": "text", "text": text }] }))
        }
        "rynix_format" => {
            let formatted = format_source(source)?;
            Ok(json!({ "content": [{ "type": "text", "text": formatted }] }))
        }
        "rynix_explain_alloc" => {
            let text = explain_source(source)?;
            Ok(json!({ "content": [{ "type": "text", "text": text }] }))
        }
        "compile" => {
            let ll = compile_source(source)?;
            Ok(json!({ "content": [{ "type": "text", "text": ll }] }))
        }
        "ast_query" => {
            let dump = ast_source(source)?;
            Ok(json!({ "content": [{ "type": "text", "text": dump }] }))
        }
        "apply_fix" => {
            let fixed = apply_fix_source(source);
            Ok(json!({ "content": [{ "type": "text", "text": fixed }] }))
        }
        "rynix_graph" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("mcp.ryx");
            let arena = AstArena::new();
            let mut parsed = crate::agent_lib::parse_text(path, source, &arena);
            if parsed.sink.error_count() > 0 {
                return Err(rpc_error(-32000, "parse/sema errors"));
            }
            let g = crate::agent_lib::graph_json_text(path, &mut parsed);
            Ok(json!({ "content": [{ "type": "text", "text": g.to_string() }] }))
        }
        "rynix_impact" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("mcp.ryx");
            let target = args.get("fn").and_then(|f| f.as_str());
            let arena = AstArena::new();
            let mut parsed = crate::agent_lib::parse_text(path, source, &arena);
            if parsed.sink.error_count() > 0 {
                return Err(rpc_error(-32000, "parse/sema errors"));
            }
            let impact = crate::agent_lib::impact_json(path, &mut parsed, target)
                .map_err(|e| rpc_error(-32000, e))?;
            Ok(json!({ "content": [{ "type": "text", "text": impact.to_string() }] }))
        }
        "rynix_precheck" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("mcp.ryx");
            let target = args.get("fn").and_then(|f| f.as_str());
            let allow_write = args
                .get("allow_write")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let arena = AstArena::new();
            let mut parsed = crate::agent_lib::parse_text(path, source, &arena);
            if parsed.sink.error_count() > 0 {
                return Err(rpc_error(-32000, "parse/sema errors"));
            }
            let impact = crate::agent_lib::impact_json(path, &mut parsed, target)
                .map_err(|e| rpc_error(-32000, e))?;
            let report = json!({
                "schema": "rynix.precheck.v1",
                "path": path,
                "write_allowed": allow_write,
                "fn": target,
                "impact": impact,
            });
            Ok(json!({ "content": [{ "type": "text", "text": report.to_string() }] }))
        }
        "rynix_context" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("mcp.ryx");
            let budget = args
                .get("budget")
                .and_then(|b| b.as_u64())
                .unwrap_or(2000) as usize;
            let budget = budget.max(1);
            let arena = AstArena::new();
            let parsed = crate::agent_lib::parse_text(path, source, &arena);
            if parsed.sink.error_count() > 0 {
                return Err(rpc_error(-32000, "parse/sema errors"));
            }
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
                "path": path,
                "budget": budget,
                "chars_used": used,
                "truncated": truncated,
                "lines": lines,
            });
            Ok(json!({ "content": [{ "type": "text", "text": report.to_string() }] }))
        }
        "rynix_security" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("mcp.ryx");
            let report = scan_source(path, source);
            Ok(json!({ "content": [{ "type": "text", "text": report.to_json().to_string() }] }))
        }
        other => Err(rpc_error(-32602, format!("unknown tool: {other}"))),
    }
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

fn rpc_error(code: i64, message: impl AsRef<str>) -> Value {
    json!({ "code": code, "message": message.as_ref() })
}

fn write_result(stdout: &mut impl Write, id: Option<Value>, result: Result<Value, Value>) {
    let mut body = json!({ "jsonrpc": "2.0" });
    if let Some(id) = id {
        body["id"] = id;
    }
    match result {
        Ok(v) => body["result"] = v,
        Err(e) => body["error"] = e,
    }
    let _ = write_message(stdout, &body);
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length")
    })?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    let v: Value = serde_json::from_slice(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(v))
}

fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let data = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", data.len())?;
    writer.write_all(&data)?;
    writer.flush()?;
    Ok(())
}
