//! `rynixc emit-ll` — lower to RIR and print / write textual LLVM IR.

use std::process::ExitCode;

use crate::cli::EmitLlOptions;
use crate::codegen_pipe;

pub fn run(options: &EmitLlOptions) -> ExitCode {
    let result = match codegen_pipe::compile_to_llvm(
        &options.path,
        options.optimize,
        options.error_format,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    if let Some(out) = &options.output {
        if let Err(e) = std::fs::write(out, &result.ll) {
            eprintln!("error: cannot write {}: {e}", out.display());
            return ExitCode::from(3);
        }
    } else {
        print!("{}", result.ll);
    }
    ExitCode::SUCCESS
}
