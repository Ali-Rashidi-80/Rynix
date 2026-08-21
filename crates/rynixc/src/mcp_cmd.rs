//! `rynixc mcp-serve` — JSON-RPC 2.0 over stdio (Content-Length framing).
//!
//! Tools: `diagnostics`/`rynix_check`, `rynix_format`, `rynix_explain_alloc`,
//! `compile`, `ast_query`, `apply_fix`.

#![allow(clippy::too_many_lines)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

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
        }
    ])
}

fn tools_call(params: &Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| rpc_error(-32602, "missing tool name"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
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
            let fixed = apply_fix_source(source)?;
            Ok(json!({ "content": [{ "type": "text", "text": fixed }] }))
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

fn apply_fix_source(source: &str) -> Result<String, Value> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);
    for d in &sink.diags {
        if let Some(fix) = d.fixes.first() {
            let mut bytes = source.as_bytes().to_vec();
            let mut edits = fix.edits.clone();
            edits.sort_by_key(|e| std::cmp::Reverse(e.span.lo()));
            for edit in edits {
                let lo = edit.span.lo() as usize;
                let hi = edit.span.hi() as usize;
                if hi > bytes.len() || lo > hi {
                    continue;
                }
                bytes.splice(lo..hi, edit.replacement.bytes());
            }
            return String::from_utf8(bytes)
                .map_err(|_| rpc_error(-32000, "fix produced non-utf8"));
        }
    }
    Ok(source.to_string())
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
