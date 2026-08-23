//! `rynixc emit-ll` — lower to RIR and print / write textual LLVM IR.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::EmitLlOptions;
use crate::codegen_pipe;
use crate::manifest::{resolve_for_source, DepsReport};

pub fn run(options: &EmitLlOptions) -> ExitCode {
    let dep_entries = match collect_compile_entries(&options.path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match codegen_pipe::compile_to_llvm_with_deps(
        &options.path,
        &dep_entries,
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

fn collect_compile_entries(source: &std::path::Path) -> Result<Vec<PathBuf>, String> {
    match resolve_for_source(source) {
        Ok(None) => Ok(Vec::new()),
        Ok(Some(report)) => entries_from_report(&report),
        Err(e) => Err(e),
    }
}

fn entries_from_report(report: &DepsReport) -> Result<Vec<PathBuf>, String> {
    if !report.all_ok() {
        let fails: Vec<_> = report
            .deps
            .iter()
            .filter(|d| !d.ok)
            .map(|d| format!("{}: {}", d.name, d.detail))
            .collect();
        return Err(format!(
            "path dependency resolve failed:\n  {}",
            fails.join("\n  ")
        ));
    }
    if report.deps.is_empty() {
        return Ok(Vec::new());
    }
    report
        .compile_entry_paths()
        .map_err(|e| format!("dependency compile failed:\n  {e}"))
}
