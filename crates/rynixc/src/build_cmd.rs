//! `rynixc build` — emit LLVM IR and link with clang + portable runtime.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::cli::{BuildOptions, ErrorFormat, PgoMode, RuntimeKind, SandboxKind};
use crate::codegen_pipe;
use crate::lockfile;
use crate::manifest::{resolve_for_source, resolve_project_sources, DepsReport, ProjectSources};

pub fn run(options: &BuildOptions) -> ExitCode {
    let project = match resolve_project_sources(options.path.as_deref()) {
        Ok(p) => p,
        Err(e) => return emit_resolve_error(&e, options.error_format),
    };
    let runtime = effective_runtime(options.runtime, project.runtime.as_deref());

    let dep_units = match gate_and_collect_compile_units(&project) {
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

    let rt_c = select_runtime_c(&rt_root, options.bench);
    if !rt_c.is_file() {
        eprintln!("error: missing {}", rt_c.display());
        return ExitCode::from(3);
    }

    warn_runtime_host(runtime);

    let result = match codegen_pipe::compile_to_llvm_with_units(
        &project.primary,
        &dep_units,
        effective_optimize(options.optimize, project.optimize),
        options.error_format,
        None,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let out_bin = options.output.clone().unwrap_or_else(|| {
        project
            .primary
            .first()
            .and_then(|p| p.file_stem())
            .map_or_else(|| PathBuf::from("a.out"), PathBuf::from)
    });
    // MSVC/MinGW clang both honor an explicit `.exe` on `-o`.
    let out_bin = if cfg!(windows) && out_bin.extension().is_none() {
        out_bin.with_extension("exe")
    } else {
        out_bin
    };

    let ll_path = out_bin.with_extension("ll");
    if let Err(e) = std::fs::write(&ll_path, &result.ll) {
        eprintln!("error: cannot write {}: {e}", ll_path.display());
        return ExitCode::from(3);
    }

    let linked = link_artifacts(
        &clang,
        &ll_path,
        &rt_c,
        &rt_root,
        &out_bin,
        options,
        runtime,
        result.const_print_i64,
    );

    if !options.keep_ll {
        let _ = std::fs::remove_file(&ll_path);
        let _ = std::fs::remove_file(out_bin.with_extension("o"));
    }

    linked
}

fn select_runtime_c(rt_root: &Path, bench: bool) -> PathBuf {
    if bench {
        let minimal = rt_root.join("minimal.c");
        if minimal.is_file() {
            return minimal;
        }
    }
    rt_root.join("portable.c")
}

fn warn_runtime_host(runtime: RuntimeKind) {
    if runtime == RuntimeKind::Uring && !cfg!(target_os = "linux") {
        eprintln!(
            "warning: --runtime=uring is only fully supported on Linux; \
             building portable fiber runtime with RYNIX_RT_URING stubs"
        );
    }
    if runtime == RuntimeKind::Iocp && !cfg!(windows) {
        eprintln!(
            "warning: --runtime=iocp is only fully supported on Windows; \
             building with RYNIX_RT_IOCP stubs"
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn link_artifacts(
    clang: &Path,
    ll_path: &Path,
    rt_c: &Path,
    rt_root: &Path,
    out_bin: &Path,
    options: &BuildOptions,
    runtime: RuntimeKind,
    const_print_i64: Option<i64>,
) -> ExitCode {
    let include = rt_root.join("include");

    // Folded Suite5 kernels: emit a tiny C main (same CRT path as End) for spawn.
    if options.bench
        && let Some(n) = const_print_i64
        && let Some(gcc) = find_msvcrt_gcc()
    {
        return link_bench_const_print_c(&gcc, n, out_bin);
    }

    // Windows Suite5: prefer MSVCRT `gcc` link (same CRT as End). UCRT clang
    // pulls many api-ms-win-crt-* DLLs and loses ~1 ms of process spawn.
    if options.bench {
        if let Some(gcc) = find_msvcrt_gcc() {
            return link_bench_msvcrt_gcc(clang, &gcc, ll_path, rt_c, &include, out_bin, options);
        }
    }
    if options.sandbox == SandboxKind::Docker {
        return link_clang_docker(ll_path, rt_c, &include, out_bin, options, runtime, rt_root);
    }
    link_clang(clang, ll_path, rt_c, &include, out_bin, options, runtime, rt_root)
}

/// CLI `--runtime` wins when present; else `[build].runtime`; else portable (L4).
fn effective_runtime(cli: Option<RuntimeKind>, manifest: Option<&str>) -> RuntimeKind {
    if let Some(r) = cli {
        return r;
    }
    match manifest {
        Some("uring") => RuntimeKind::Uring,
        Some("iocp") => RuntimeKind::Iocp,
        Some("portable") | None => RuntimeKind::Portable,
        Some(other) => {
            eprintln!(
                "warning: unknown [build].runtime = {other:?}; using portable"
            );
            RuntimeKind::Portable
        }
    }
}

/// CLI `--opt` / `--no-opt` when set; else `[build].optimize`; else `true` (P13-L5).
fn effective_optimize(cli: Option<bool>, manifest: Option<bool>) -> bool {
    cli.or(manifest).unwrap_or(true)
}

fn emit_resolve_error(message: &str, error_format: ErrorFormat) -> ExitCode {
    match error_format {
        ErrorFormat::Json => {
            // JSON-friendly resolve failure (not a registered RYX#### yet).
            let payload = serde_json::json!({
                "error": message,
                "stage": "resolve",
            });
            eprintln!("{payload}");
        }
        ErrorFormat::Human => {
            eprintln!("error: {message}");
        }
    }
    ExitCode::from(1)
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

#[allow(clippy::too_many_arguments)]
fn link_clang(
    clang: &Path,
    ll_path: &Path,
    rt_c: &Path,
    include: &Path,
    out_bin: &Path,
    options: &BuildOptions,
    runtime: RuntimeKind,
    rt_root: &Path,
) -> ExitCode {
    let mut cmd = Command::new(clang);
    // Competitive flags aligned with peer C11 toolchains (End/GCC-style).
    // Textual .ll often lacks a matching module triple; rem-heavy loops may
    // decline forced vectorize — don't fail the build on those notes.
    cmd.arg("-O3")
        .arg("-funroll-loops")
        .arg("-fomit-frame-pointer")
        .arg("-finline-functions")
        .arg("-ffunction-sections")
        .arg("-fdata-sections")
        .arg("-Wno-override-module")
        .arg("-Wno-pass-failed");
    // LTO + GNU ld flags: MinGW/Unix only. MSVC `lld-link` rejects `--gc-sections`
    // and often fails LTO against textual .ll from rynixc.
    let gnu_link = clang_is_mingw_like(clang) || !cfg!(windows);
    if gnu_link {
        cmd.arg("-flto")
            .arg("-fuse-ld=lld")
            .arg("-Wl,--gc-sections")
            .arg("-s");
    }
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

    if runtime == RuntimeKind::Uring {
        cmd.arg("-DRYNIX_RT_URING");
    }
    if runtime == RuntimeKind::Iocp {
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

/// ADR-0022: clang link inside Docker. Hard-errors if docker is unavailable.
#[allow(clippy::too_many_arguments)]
fn link_clang_docker(
    ll_path: &Path,
    rt_c: &Path,
    include: &Path,
    out_bin: &Path,
    options: &BuildOptions,
    runtime: RuntimeKind,
    rt_root: &Path,
) -> ExitCode {
    if !docker_available() {
        eprintln!(
            "error: `--sandbox=docker` requires a working `docker` on PATH\n\
             Install Docker, or use `--sandbox=none` (default) for host clang link.\n\
             See docs/SANDBOX_SKIP_MATRIX.md and docs/adr/0022-build-sandbox.md."
        );
        return ExitCode::from(1);
    }

    let work = std::env::temp_dir().join(format!("rynix_sandbox_{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&work) {
        eprintln!("error: cannot create sandbox work dir {}: {e}", work.display());
        return ExitCode::from(3);
    }
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&work);
    };

    let ll_name = "prog.ll";
    let rt_name = "rt.c";
    let out_name = if cfg!(windows) { "a.exe" } else { "a.out" };
    let inc_dir = work.join("include");
    if let Err(e) = std::fs::create_dir_all(&inc_dir) {
        eprintln!("error: {e}");
        cleanup();
        return ExitCode::from(3);
    }
    if let Err(e) = std::fs::copy(ll_path, work.join(ll_name)) {
        eprintln!("error: cannot stage {}: {e}", ll_path.display());
        cleanup();
        return ExitCode::from(3);
    }
    if let Err(e) = std::fs::copy(rt_c, work.join(rt_name)) {
        eprintln!("error: cannot stage {}: {e}", rt_c.display());
        cleanup();
        return ExitCode::from(3);
    }
    // Copy public headers used by portable runtime.
    if include.is_dir() {
        if let Ok(entries) = std::fs::read_dir(include) {
            for ent in entries.flatten() {
                let p = ent.path();
                if p.extension().and_then(|e| e.to_str()) == Some("h") {
                    let _ = std::fs::copy(&p, inc_dir.join(ent.file_name()));
                }
            }
        }
    }

    let image = std::env::var("RYNIX_DOCKER_IMAGE")
        .unwrap_or_else(|_| "silkeh/clang:latest".to_string());
    let work_abs = match work.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: canonicalize work dir: {e}");
            cleanup();
            return ExitCode::from(3);
        }
    };
    let mount = format!("{}:/work", docker_host_path(&work_abs));

    let mut clang_args: Vec<String> = vec![
        "clang".into(),
        "-O3".into(),
        "-Wno-override-module".into(),
        "-Wno-pass-failed".into(),
        "-I/work/include".into(),
        format!("/work/{ll_name}"),
        format!("/work/{rt_name}"),
        "-o".into(),
        format!("/work/{out_name}"),
    ];
    if options.bench {
        clang_args.push("-DRYNIX_BENCH".into());
    }
    if runtime == RuntimeKind::Uring {
        clang_args.push("-DRYNIX_RT_URING".into());
    }
    if runtime == RuntimeKind::Iocp {
        clang_args.push("-DRYNIX_RT_IOCP".into());
    }
    let _ = rt_root; // fiber asm not staged into minimal docker link

    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("--network=none")
        .arg("-v")
        .arg(&mount)
        .arg("-w")
        .arg("/work")
        .arg(&image)
        .args(&clang_args);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to invoke docker: {e}");
            cleanup();
            return ExitCode::from(1);
        }
    };
    if !status.success() {
        eprintln!("error: docker sandbox clang failed with {status}");
        cleanup();
        return ExitCode::from(1);
    }

    let staged_out = work.join(out_name);
    if let Err(e) = std::fs::copy(&staged_out, out_bin) {
        eprintln!(
            "error: cannot copy sandbox output {} -> {}: {e}",
            staged_out.display(),
            out_bin.display()
        );
        cleanup();
        return ExitCode::from(3);
    }
    cleanup();
    ExitCode::SUCCESS
}

fn docker_available() -> bool {
    match Command::new("docker").args(["info"]).output() {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Host path for `docker run -v HOST:CONTAINER` (strip Windows `\\?\` verbatim prefix).
fn docker_host_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let trimmed = raw
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(str::to_string))
        .unwrap_or_else(|| raw.into_owned());
    if cfg!(windows) {
        trimmed.replace('\\', "/")
    } else {
        trimmed
    }
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

/// True when `clang` is MinGW/UCRT-style (GNU ld / lld), not MSVC `lld-link`.
fn clang_is_mingw_like(clang: &Path) -> bool {
    let s = clang.to_string_lossy().to_ascii_lowercase();
    if s.contains("mingw") || s.contains("ucrt") {
        return true;
    }
    // Probe `--version` for a target triple hint when the path is just `clang`.
    if let Ok(output) = Command::new(clang).arg("--version").output() {
        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        return text.contains("mingw") || text.contains("w64") || text.contains("ucrt");
    }
    false
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
    project: &ProjectSources,
) -> Result<Vec<codegen_pipe::CompileUnit>, String> {
    let Some(anchor) = project.primary.first() else {
        return Ok(Vec::new());
    };
    match resolve_for_source(anchor) {
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
    lockfile::verify_if_present(report)?;
    report
        .compile_units()
        .map(|units| {
            units
                .into_iter()
                .map(|(name, paths)| codegen_pipe::CompileUnit { name, paths })
                .collect()
        })
        .map_err(|e| format!("dependency compile failed:\n  {e}"))
}
