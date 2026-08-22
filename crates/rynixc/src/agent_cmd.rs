//! `rynixc graph` / `slice` / `impact` — AI-agent structure exports.

use std::process::ExitCode;

use rynix_ast::{AstArena, Item, Stmt};

use crate::agent_lib::{graph_json, impact_json, parse_file};
use crate::cli::{AgentOptions, ErrorFormat, ImpactOptions};
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
    if matches!(options.error_format, ErrorFormat::Json) {
        let out = serde_json::json!({
            "schema": "rynix.slice.v1",
            "path": options.path.display().to_string(),
            "lines": lines,
        });
        println!("{out}");
    } else {
        for line in lines {
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
