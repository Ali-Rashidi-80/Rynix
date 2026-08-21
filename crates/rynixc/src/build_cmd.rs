//! `rynixc build` — emit LLVM IR and link with clang + portable runtime.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use crate::cli::BuildOptions;
use crate::codegen_pipe;

pub fn run(options: &BuildOptions) -> ExitCode {
    let Some(clang) = find_clang() else {
        eprintln!(
            "error: `clang` not found on PATH\n\
             `rynixc build` needs an external Clang/LLVM toolchain (ADR-0005).\n\
             Install LLVM and retry, or run `rynixc emit-ll` to inspect IR only."
        );
        return ExitCode::from(1);
    };

    let Some(rt) = codegen_pipe::portable_runtime_c() else {
        eprintln!("error: cannot locate rt/portable.c (runtime stubs)");
        return ExitCode::from(1);
    };

    let result = match codegen_pipe::compile_to_llvm(&options.path, true, options.error_format) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let out_bin = options.output.clone().unwrap_or_else(|| {
        options
            .path
            .file_stem()
            .map_or_else(|| PathBuf::from("a.out"), PathBuf::from)
    });

    let ll_path = out_bin.with_extension("ll");
    if let Err(e) = std::fs::write(&ll_path, &result.ll) {
        eprintln!("error: cannot write {}: {e}", ll_path.display());
        return ExitCode::from(3);
    }

    let status = match Command::new(&clang)
        .arg("-O3")
        .arg("-flto=thin")
        .arg("-ffunction-sections")
        .arg("-fuse-ld=lld")
        .arg("-Wl,--gc-sections")
        .arg(&ll_path)
        .arg(&rt)
        .arg("-o")
        .arg(&out_bin)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to invoke {}: {e}", clang.display());
            return ExitCode::from(1);
        }
    };

    if !status.success() {
        eprintln!("error: clang failed with {status}");
        return ExitCode::from(1);
    }

    if !options.keep_ll {
        let _ = std::fs::remove_file(&ll_path);
    }

    ExitCode::SUCCESS
}

fn find_clang() -> Option<PathBuf> {
    // Prefer MinGW-targeted clang on Windows so C headers resolve without MSVC.
    for name in [
        "x86_64-w64-mingw32-clang",
        "x86_64-w64-mingw32-clang.exe",
        "clang",
        "clang.exe",
        "clang-20",
        "clang-19",
        "clang-18",
    ] {
        if let Ok(output) = Command::new(name).arg("--version").output()
            && output.status.success()
        {
            return Some(PathBuf::from(name));
        }
    }
    if let Ok(cc) = std::env::var("CC") {
        let p = PathBuf::from(&cc);
        if p.file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.contains("clang"))
        {
            return Some(p);
        }
    }
    None
}
