//! The Rynix compiler driver.
//!
//! ```text
//! rynixc lex | parse | check | dump-rir | emit-ll | emit-wasm | build | run | test | fmt
//!         | graph | slice | impact | eval | patch | verify | precheck | context
//!         | security | scope | deps | dna | new | mcp-serve | lsp-serve | arch
//! ```

mod agent_cmd;
mod agent_lib;
mod arch_cmd;
mod architecture;
mod build_cmd;
mod check_cmd;
mod cli;
mod codegen_pipe;
mod contract;
mod deps_cmd;
mod dna;
mod dna_cmd;
mod driver;
mod dump_rir_cmd;
mod emit_ll_cmd;
mod emit_wasm_cmd;
mod eval_cmd;
mod fix;
mod fmt_cmd;
mod lex_cmd;
mod lockfile;
mod lsp_cmd;
mod manifest;
mod mcp_cmd;
mod new_cmd;
mod parse_cmd;
mod patch_cmd;
mod run_cmd;
mod scope;
mod scope_cmd;
mod security;
mod security_cmd;
mod test_cmd;
mod verify_cmd;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Version) => {
            println!("rynixc {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Lex(options)) => lex_cmd::run(&options),
        Ok(cli::Command::Parse(options)) => parse_cmd::run(&options),
        Ok(cli::Command::Check(options)) => check_cmd::run(&options),
        Ok(cli::Command::DumpRir(options)) => dump_rir_cmd::run(&options),
        Ok(cli::Command::EmitLl(options)) => emit_ll_cmd::run(&options),
        Ok(cli::Command::EmitWasm(options)) => emit_wasm_cmd::run(&options),
        Ok(cli::Command::Build(options)) => build_cmd::run(&options),
        Ok(cli::Command::Run(options)) => run_cmd::run(&options),
        Ok(cli::Command::Test(options)) => test_cmd::run(&options),
        Ok(cli::Command::Fmt(options)) => fmt_cmd::run(&options),
        Ok(cli::Command::Graph(options)) => agent_cmd::run_graph(&options),
        Ok(cli::Command::Slice(options)) => agent_cmd::run_slice(&options),
        Ok(cli::Command::Impact(options)) => agent_cmd::run_impact(&options),
        Ok(cli::Command::Eval(options)) => eval_cmd::run(&options),
        Ok(cli::Command::Patch(options)) => patch_cmd::run(&options),
        Ok(cli::Command::Verify(options)) => verify_cmd::run(&options),
        Ok(cli::Command::Precheck(options)) => agent_cmd::run_precheck(&options),
        Ok(cli::Command::Context(options)) => agent_cmd::run_context(&options),
        Ok(cli::Command::Security(options)) => security_cmd::run(&options),
        Ok(cli::Command::Scope(options)) => scope_cmd::run(&options),
        Ok(cli::Command::Deps(options)) => deps_cmd::run(&options),
        Ok(cli::Command::Dna(options)) => dna_cmd::run(&options),
        Ok(cli::Command::New(options)) => new_cmd::run(&options),
        Ok(cli::Command::McpServe) => mcp_cmd::run(),
        Ok(cli::Command::LspServe) => lsp_cmd::run(),
        Ok(cli::Command::ArchCheck(options)) => arch_cmd::run(&options),
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}
