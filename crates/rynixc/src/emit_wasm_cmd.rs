//! `rynixc emit-wasm` — wasm32 LLVM IR linked to a real `.wasm` via clang.
//!
//! No WASI libc and no host `rt/` link (Phase 14 Wave A). Soft-runtime declares
//! may appear in the `.ll`; they must remain uncalled for a successful nostdlib
//! link of arith-only programs. `print_i64` is a host import (`env.print_i64`)
//! so Node (or another host) can supply it without WASI (Phase 20-A).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::cli::EmitWasmOptions;
use crate::codegen_pipe::{self, CompileUnit};
use crate::lockfile;
use crate::manifest::{resolve_for_source, DepsReport};

const WASM_TRIPLE: &str = "wasm32-unknown-unknown";

pub fn run(options: &EmitWasmOptions) -> ExitCode {
    let Some(clang) = find_clang_wasm_link() else {
        eprintln!(
            "error: no clang on PATH that can link `--target={WASM_TRIPLE}`\n\
             Install an LLVM with the wasm32 target, or run \
             `rynixc emit-ll --target={WASM_TRIPLE}` for IR only."
        );
        return ExitCode::from(1);
    };

    let dep_units = match collect_compile_units(&options.path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match codegen_pipe::compile_to_llvm_with_units(
        std::slice::from_ref(&options.path),
        &dep_units,
        options.optimize,
        options.error_format,
        Some(WASM_TRIPLE),
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let out = options.output.clone().unwrap_or_else(|| {
        let stem = options
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("out");
        PathBuf::from(format!("{stem}.wasm"))
    });

    let tmp_dir = std::env::temp_dir();
    let ll_path = tmp_dir.join(format!(
        "rynix_emit_wasm_{}.ll",
        std::process::id()
    ));
    if let Err(e) = std::fs::write(&ll_path, &result.ll) {
        eprintln!("error: cannot write {}: {e}", ll_path.display());
        return ExitCode::from(3);
    }

    let status = link_wasm(&clang, &ll_path, &out);
    let _ = std::fs::remove_file(&ll_path);
    match status {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

/// Prefer a clang that can freestanding-link wasm32 (not host MinGW-first).
fn find_clang_wasm_link() -> Option<PathBuf> {
    let candidates = [
        "clang",
        "clang.exe",
        "clang-20",
        "clang-19",
        "clang-18",
        "x86_64-w64-mingw32-clang",
        "x86_64-w64-mingw32-clang.exe",
    ];
    let probe_ll = std::env::temp_dir().join("rynix_wasm_link_probe.ll");
    let probe_wasm = std::env::temp_dir().join("rynix_wasm_link_probe.wasm");
    let _ = std::fs::write(
        &probe_ll,
        "target triple = \"wasm32-unknown-unknown\"\ndefine i32 @main() {\nentry:\n  ret i32 0\n}\n",
    );
    let Some(ll) = probe_ll.to_str() else {
        return None;
    };
    let Some(wasm) = probe_wasm.to_str() else {
        return None;
    };
    let target = format!("--target={WASM_TRIPLE}");
    for name in candidates {
        if Command::new(name).arg("--version").output().is_err() {
            continue;
        }
        let ok = Command::new(name)
            .args([
                target.as_str(),
                "-nostdlib",
                "-Wl,--no-entry",
                "-Wl,--export-all",
                "-o",
                wasm,
                ll,
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            let _ = std::fs::remove_file(&probe_wasm);
            return Some(PathBuf::from(name));
        }
    }
    None
}

fn link_wasm(clang: &Path, ll_path: &Path, out: &Path) -> Result<(), ExitCode> {
    let mut cmd = Command::new(clang);
    cmd.arg(format!("--target={WASM_TRIPLE}"))
        .arg("-nostdlib")
        .arg("-Wl,--no-entry")
        .arg("-Wl,--export-all")
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(out)
        .arg(ll_path);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: failed to invoke {}: {e}", clang.display());
            return Err(ExitCode::from(1));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "error: clang wasm link failed (no WASI / no rt/ in v1)\n{stderr}"
        );
        return Err(ExitCode::from(1));
    }
    Ok(())
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
