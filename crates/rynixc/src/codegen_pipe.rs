//! Shared front-end → RIR pipeline for codegen subcommands.

use std::path::Path;
use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_codegen::{emit_llvm, prune_unreachable};
use rynix_diag::VecSink;
use rynix_rir::{analyze_escape, inject_regions, lower_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};

use crate::cli::ErrorFormat;
use crate::driver;

pub struct CodegenResult {
    pub ll: String,
}

pub fn compile_to_llvm(
    path: &Path,
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    let mut sources = SourceMap::new();
    let file_id = match sources.load_file(path) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", path.display());
            return Err(ExitCode::from(3));
        }
    };

    let file = sources.file(file_id);
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(
        &arena,
        &mut interner,
        file.text(),
        file.start_pos(),
        &mut sink,
    );
    let analysis = analyze(module, &mut interner, &mut sink);
    if sink.error_count() > 0 {
        return Err(driver::emit_diagnostics(&sink, &sources, error_format));
    }

    let mut rir = lower_module(
        module,
        &analysis,
        &mut interner,
        file.text(),
        file.start_pos(),
    );
    if optimize {
        let errs = run_pipeline(&mut rir);
        if !errs.is_empty() {
            for e in &errs {
                eprintln!("rir verifier: {e}");
            }
            return Err(ExitCode::from(1));
        }
    }

    let report = analyze_escape(&rir, &interner);
    inject_regions(&mut rir, &report);
    prune_unreachable(&mut rir, &interner);
    let ll = emit_llvm(&rir, &interner, Some(&report));

    Ok(CodegenResult { ll })
}

/// Locate `rt/portable.c` relative to the compiler binary / source tree.
pub fn portable_runtime_c() -> Option<std::path::PathBuf> {
    // 1) Next to the executable: <exe_dir>/rt/portable.c or ../rt/portable.c
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidates = [
            dir.join("rt/portable.c"),
            dir.join("../rt/portable.c"),
            dir.join("../../rt/portable.c"),
        ];
        for c in candidates {
            if c.is_file() {
                return Some(c);
            }
        }
    }
    // 2) Workspace layout when running via `cargo run`
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws = manifest.join("../../rt/portable.c");
    if ws.is_file() {
        return Some(ws.canonicalize().unwrap_or(ws));
    }
    let ws2 = manifest.join("../rt/portable.c");
    if ws2.is_file() {
        return Some(ws2);
    }
    None
}
