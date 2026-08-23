//! `rynixc graph` / `slice` / `impact` — AI-agent structure exports.

use std::process::ExitCode;

use rynix_ast::AstArena;

use crate::agent_lib::{graph_json, impact_json, parse_file, slice_lines};
use crate::cli::{
    AgentOptions, ContextOptions, ErrorFormat, ImpactOptions, PrecheckOptions,
};
use crate::driver;

pub fn run_graph(options: &AgentOptions) -> ExitCode {
    let arena = AstArena::new();
    let mut parsed = match parse_file(&options.path, &arena) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let code = driver::emit_diagnostics(&parsed.sink, &parsed.sources, options.error_format);
    if code != ExitCode::SUCCESS {
        return code;
    }
    println!("{}", graph_json(&options.path, &mut parsed));
    ExitCode::SUCCESS
}

pub fn run_slice(options: &AgentOptions) -> ExitCode {
    let arena = AstArena::new();
    let parsed = match parse_file(&options.path, &arena) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let code = driver::emit_diagnostics(&parsed.sink, &parsed.sources, options.error_format);
    if code != ExitCode::SUCCESS {
        return code;
    }

    let mut lines = slice_lines(&parsed);
    if matches!(options.error_format, ErrorFormat::Json) {
        let out = serde_json::json!({
            "schema": "rynix.slice.v1",
            "path": options.path.display().to_string(),
            "lines": lines,
        });
        println!("{out}");
    } else {
        for line in lines.drain(..) {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

pub fn run_impact(options: &ImpactOptions) -> ExitCode {
    let arena = AstArena::new();
    let mut parsed = match parse_file(&options.path, &arena) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let code = driver::emit_diagnostics(&parsed.sink, &parsed.sources, options.error_format);
    if code != ExitCode::SUCCESS {
        return code;
    }
    let path = options.path.display().to_string();
    match impact_json(&path, &mut parsed, options.function.as_deref()) {
        Ok(v) => {
            println!("{v}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_precheck(options: &PrecheckOptions) -> ExitCode {
    let arena = AstArena::new();
    let mut parsed = match parse_file(&options.path, &arena) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let code = driver::emit_diagnostics(&parsed.sink, &parsed.sources, options.error_format);
    if code != ExitCode::SUCCESS {
        return code;
    }
    let path = options.path.display().to_string();
    let impact = match impact_json(&path, &mut parsed, options.function.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let report = serde_json::json!({
        "schema": "rynix.precheck.v1",
        "path": path,
        "write_allowed": options.allow_write,
        "fn": options.function,
        "impact": impact,
    });
    match options.error_format {
        ErrorFormat::Human => {
            println!(
                "precheck: write_allowed={} path={}",
                options.allow_write, options.path.display()
            );
            if let Some(f) = &options.function {
                println!("  focus fn: {f}");
            }
            println!("{impact}");
        }
        ErrorFormat::Json => println!("{report}"),
    }
    ExitCode::SUCCESS
}

pub fn run_context(options: &ContextOptions) -> ExitCode {
    let arena = AstArena::new();
    let parsed = match parse_file(&options.path, &arena) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(3);
        }
    };
    let code = driver::emit_diagnostics(&parsed.sink, &parsed.sources, options.error_format);
    if code != ExitCode::SUCCESS {
        return code;
    }
    let all = slice_lines(&parsed);
    let mut lines = Vec::new();
    let mut used = 0usize;
    let mut truncated = false;
    for line in all {
        let add = if lines.is_empty() {
            line.len()
        } else {
            line.len() + 1
        };
        if used + add > options.budget {
            truncated = true;
            break;
        }
        used += add;
        lines.push(line);
    }
    let report = serde_json::json!({
        "schema": "rynix.context.v1",
        "path": options.path.display().to_string(),
        "budget": options.budget,
        "chars_used": used,
        "truncated": truncated,
        "lines": lines.clone(),
    });
    match options.error_format {
        ErrorFormat::Human => {
            for line in &lines {
                println!("{line}");
            }
            if truncated {
                eprintln!(
                    "# truncated: chars_used={used} budget={}",
                    options.budget
                );
            }
        }
        ErrorFormat::Json => println!("{report}"),
    }
    ExitCode::SUCCESS
}
