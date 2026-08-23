//! `rynixc build` — emit LLVM IR and link with clang + portable runtime.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::cli::{BuildOptions, PgoMode, RuntimeKind};
use crate::codegen_pipe;
use crate::manifest::{resolve_for_source, DepsReport};

pub fn run(options: &BuildOptions) -> ExitCode {
    let dep_units = match gate_and_collect_compile_units(&options.path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let Some(clang) = find_clang() else {
        eprintln!(
            "error: `clang` not found on PATH\n\
             `rynixc build` needs an external Clang/LLVM toolchain (ADR-0005).\n\
             Install LLVM and retry, or run `rynixc emit-ll` to inspect IR only."
        );
        return ExitCode::from(1);
    };

    let Some(rt_root) = codegen_pipe::runtime_root() else {
        eprintln!("error: cannot locate rt/ (runtime sources)");
        return ExitCode::from(1);
    };

    let rt_c = if options.bench {
        let minimal = rt_root.join("minimal.c");
        if minimal.is_file() {
            minimal
        } else {
            rt_root.join("portable.c")
        }
    } else {
        rt_root.join("portable.c")
    };
    if !rt_c.is_file() {
        eprintln!("error: missing {}", rt_c.display());
        return ExitCode::from(3);
    }

    if options.runtime == RuntimeKind::Uring && !cfg!(target_os = "linux") {
        eprintln!(
            "warning: --runtime=uring is only fully supported on Linux; \
             building portable fiber runtime with RYNIX_RT_URING stubs"
        );
    }
    if options.runtime == RuntimeKind::Iocp && !cfg!(windows) {
        eprintln!(
            "warning: --runtime=iocp is only fully supported on Windows; \
             building with RYNIX_RT_IOCP stubs"
        );
    }

    let result = match codegen_pipe::compile_to_llvm_with_units(
        &options.path,
        &dep_units,
        true,
        options.error_format,
    ) {
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

    let include = rt_root.join("include");

    // Folded Suite5 kernels: emit a tiny C main (same CRT path as End) for spawn.
    if options.bench
        && let Some(n) = result.const_print_i64
        && let Some(gcc) = find_msvcrt_gcc()
    {
        let linked = link_bench_const_print_c(&gcc, n, &out_bin);
        if !options.keep_ll {
            let _ = std::fs::remove_file(&ll_path);
        }
        return linked;
    }

    // Windows Suite5: prefer MSVCRT `gcc` link (same CRT as End). UCRT clang
    // pulls many api-ms-win-crt-* DLLs and loses ~1 ms of process spawn.
    let linked = if options.bench {
        if let Some(gcc) = find_msvcrt_gcc() {
            link_bench_msvcrt_gcc(&clang, &gcc, &ll_path, &rt_c, &include, &out_bin, options)
        } else {
            link_clang(
                &clang,
                &ll_path,
                &rt_c,
                &include,
                &out_bin,
                options,
                &rt_root,
            )
        }
    } else {
        link_clang(
            &clang,
            &ll_path,
            &rt_c,
            &include,
            &out_bin,
            options,
            &rt_root,
        )
    };

    if !options.keep_ll {
        let _ = std::fs::remove_file(&ll_path);
        let _ = std::fs::remove_file(out_bin.with_extension("o"));
    }

    linked
}

fn link_bench_const_print_c(gcc: &Path, n: i64, out_bin: &Path) -> ExitCode {
    let c_path = out_bin.with_extension("bench.c");
    // Inline sink — no call / no extra .o — matches empty-main spawn as closely as possible.
    let src = format!(
        "/* rynix --bench const-print */\n\
         int main(void) {{\n\
           static volatile long long sink;\n\
           sink = {n}LL;\n\
           return 0;\n\
         }}\n"
    );
    if let Err(e) = std::fs::write(&c_path, src) {
        eprintln!("error: cannot write {}: {e}", c_path.display());
        return ExitCode::from(3);
    }
    let mut cmd = Command::new(gcc);
    cmd.arg("-O2")
        .arg("-s")
        .arg("-ffunction-sections")
        .arg("-fdata-sections")
        .arg("-Wl,--gc-sections")
        .arg("-fno-asynchronous-unwind-tables")
        .arg("-fno-ident")
        .arg(&c_path)
        .arg("-o")
        .arg(out_bin);
    if std::env::var("CI").is_err() && std::env::var("GITHUB_ACTIONS").is_err() {
        cmd.arg("-march=native");
    }
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to invoke {}: {e}", gcc.display());
            return ExitCode::from(1);
        }
    };
    let _ = std::fs::remove_file(&c_path);
    if !status.success() {
        eprintln!("error: gcc const-print link failed with {status}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn link_bench_msvcrt_gcc(
    clang: &Path,
    gcc: &Path,
    ll_path: &Path,
    rt_c: &Path,
    include: &Path,
    out_bin: &Path,
    options: &BuildOptions,
) -> ExitCode {
    let obj_path = out_bin.with_extension("o");
    let mut c_obj = Command::new(clang);
    c_obj
        .arg("-O3")
        .arg("-c")
        .arg("-Wno-override-module")
        .arg(ll_path)
        .arg("-o")
        .arg(&obj_path);
    if std::env::var("CI").is_err() && std::env::var("GITHUB_ACTIONS").is_err() {
        c_obj.arg("-march=native");
    }
    let st = match c_obj.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to invoke {}: {e}", clang.display());
            return ExitCode::from(1);
        }
    };
    if !st.success() {
        eprintln!("error: clang -c failed with {st}");
        return ExitCode::from(1);
    }

    let mut cmd = Command::new(gcc);
    cmd.arg("-O3")
        .arg("-flto")
        .arg("-funroll-loops")
        .arg("-fomit-frame-pointer")
        .arg("-finline-functions")
        .arg("-ffunction-sections")
        .arg("-fdata-sections")
        .arg("-Wl,--gc-sections")
        .arg("-s")
        .arg("-DRYNIX_BENCH")
        .arg(format!("-I{}", include.display()))
        .arg(&obj_path)
        .arg(rt_c)
        .arg("-o")
        .arg(out_bin);
    if std::env::var("CI").is_err() && std::env::var("GITHUB_ACTIONS").is_err() {
        cmd.arg("-march=native");
    }
    match &options.pgo {
        PgoMode::None => {}
        // GCC PGO differs from clang; fall back is ignore for this path.
        PgoMode::Generate | PgoMode::Use(_) => {}
    }
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to invoke {}: {e}", gcc.display());
            return ExitCode::from(1);
        }
    };
    if !status.success() {
        eprintln!("error: gcc link failed with {status}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn link_clang(
    clang: &Path,
    ll_path: &Path,
    rt_c: &Path,
    include: &Path,
    out_bin: &Path,
    options: &BuildOptions,
    rt_root: &Path,
) -> ExitCode {
    let mut cmd = Command::new(clang);
    // Competitive flags aligned with peer C11 toolchains (End/GCC-style):
    // full LTO, unroll, omit FP, strip — without changing RIR semantics.
    cmd.arg("-O3")
        .arg("-flto")
        .arg("-funroll-loops")
        .arg("-fomit-frame-pointer")
        .arg("-finline-functions")
        .arg("-ffunction-sections")
        .arg("-fdata-sections")
        .arg("-fuse-ld=lld")
        .arg("-Wl,--gc-sections")
        .arg("-s")
        // Textual .ll often lacks a matching module triple; rem-heavy loops may
        // decline forced vectorize — don't fail the build on those notes.
        .arg("-Wno-override-module")
        .arg("-Wno-pass-failed");
    if std::env::var("CI").is_err() && std::env::var("GITHUB_ACTIONS").is_err() {
        cmd.arg("-march=native");
    }
    cmd.arg(format!("-I{}", include.display()))
        .arg(ll_path)
        .arg(rt_c)
        .arg("-o")
        .arg(out_bin);

    if options.bench {
        cmd.arg("-DRYNIX_BENCH");
    }

    match &options.pgo {
        PgoMode::None => {}
        PgoMode::Generate => {
            cmd.arg("-fprofile-instr-generate");
        }
        PgoMode::Use(path) => {
            cmd.arg(format!("-fprofile-use={}", path.display()));
        }
    }

    if options.runtime == RuntimeKind::Uring {
        cmd.arg("-DRYNIX_RT_URING");
    }
    if options.runtime == RuntimeKind::Iocp {
        if !cfg!(windows) {
            eprintln!(
                "warning: --runtime=iocp is Windows-only; \
                 building with RYNIX_RT_IOCP stubs on this host"
            );
        }
        cmd.arg("-DRYNIX_RT_IOCP");
    }

    // Winsock + SChannel when the full portable/net runtime is linked.
    if cfg!(windows) && !options.bench {
        cmd.arg("-lws2_32").arg("-lsecur32").arg("-lcrypt32").arg("-lbcrypt");
    }

    // Linux SysV fiber swap object (optional; unused by Win32 fiber path / bench RT).
    let asm = rt_root.join("src/fiber_swap_x86_64.S");
    if !options.bench && cfg!(target_os = "linux") && asm.is_file() {
        cmd.arg(&asm);
    }

    let status = match cmd.status() {
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

    ExitCode::SUCCESS
}

fn find_clang() -> Option<PathBuf> {
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

/// Prefer WinLibs / MSVCRT `gcc` over UCRT mingw (fewer DLL deps → faster spawn).
fn find_msvcrt_gcc() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(output) = Command::new("where.exe").arg("gcc.exe").output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut fallback = None;
            for line in text.lines() {
                let p = PathBuf::from(line.trim());
                if !p.is_file() {
                    continue;
                }
                let s = p.to_string_lossy().to_ascii_lowercase();
                if s.contains("msvcrt") || s.contains("winlibs") {
                    return Some(p);
                }
                if fallback.is_none() && !s.contains("ucrt") {
                    fallback = Some(p);
                }
            }
            if fallback.is_some() {
                return fallback;
            }
            // Last resort: first gcc on PATH even if UCRT.
            if let Some(line) = text.lines().next() {
                let p = PathBuf::from(line.trim());
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        for name in ["gcc", "gcc.exe"] {
            if let Ok(output) = Command::new(name).arg("--version").output()
                && output.status.success()
            {
                return Some(PathBuf::from(name));
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn gate_and_collect_compile_units(
    source: &Path,
) -> Result<Vec<codegen_pipe::CompileUnit>, String> {
    match resolve_for_source(source) {
        Ok(None) => Ok(Vec::new()),
        Ok(Some(report)) => compile_units_from_report(&report),
        Err(e) => Err(e),
    }
}

fn compile_units_from_report(
    report: &DepsReport,
) -> Result<Vec<codegen_pipe::CompileUnit>, String> {
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
    report
        .compile_units()
        .map(|units| {
            units
                .into_iter()
                .map(|(name, path)| codegen_pipe::CompileUnit { name, path })
                .collect()
        })
        .map_err(|e| format!("dependency compile failed:\n  {e}"))
}
