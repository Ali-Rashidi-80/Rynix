//! `rynixc emit-ll` — lower to RIR and print / write textual LLVM IR.

use std::process::ExitCode;

use crate::cli::EmitLlOptions;
use crate::codegen_pipe::{self, CompileUnit};
use crate::lockfile;
use crate::manifest::{resolve_for_source, DepsReport};

pub fn run(options: &EmitLlOptions) -> ExitCode {
    let dep_units = match collect_compile_units(&options.path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match codegen_pipe::compile_to_llvm_with_units(
        &options.path,
        &dep_units,
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

fn collect_compile_units(
    source: &std::path::Path,
) -> Result<Vec<CompileUnit>, String> {
    match resolve_for_source(source) {
        Ok(None) => Ok(Vec::new()),
        Ok(Some(report)) => units_from_report(&report),
        Err(e) => Err(e),
    }
}

fn units_from_report(report: &DepsReport) -> Result<Vec<CompileUnit>, String> {
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
    lockfile::verify_if_present(report)?;
    if report.deps.is_empty() {
        return Ok(Vec::new());
    }
    report
        .compile_units()
        .map(|units| {
            units
                .into_iter()
                .map(|(name, paths)| CompileUnit { name, paths })
                .collect()
        })
        .map_err(|e| format!("dependency compile failed:\n  {e}"))
}
