//! Shared front-end → RIR pipeline for codegen subcommands.

use std::path::Path;
use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_codegen::{emit_llvm, prune_unreachable};
use rynix_diag::VecSink;
use rynix_rir::{
    analyze_escape, inject_regions, interpret_module_print, lower_module, run_pipeline, Inst,
    Module,
};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};

use crate::cli::ErrorFormat;
use crate::driver;

pub struct CodegenResult {
    pub ll: String,
    /// When main only prints a folded i64 constant (no loops), Suite5 `--bench`
    /// can emit a tiny C TU for End-competitive process spawn.
    pub const_print_i64: Option<i64>,
}

/// Compile one primary `.ryx` (no package deps).
#[allow(dead_code)] // public helper; build/emit-ll use `compile_to_llvm_with_deps`
pub fn compile_to_llvm(
    path: &Path,
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    compile_to_llvm_with_deps(path, &[], optimize, error_format)
}

/// Unity-compile primary + dependency `entry` files into one LLVM unit (SPEC §6.3).
///
/// Flat symbol namespace: `def` names from deps are visible to the app without
/// `import`. Dependency units must not define `main`.
pub fn compile_to_llvm_with_deps(
    primary: &Path,
    dep_entries: &[std::path::PathBuf],
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    let (unity_name, unity_text) = match build_unity_source(primary, dep_entries) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return Err(ExitCode::from(1));
        }
    };

    let mut sources = SourceMap::new();
    let file_id = sources.add_owned(unity_name, unity_text);
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
    let const_print_i64 = detect_const_print_i64(&ll)
        .or_else(|| eval_const_print_if_acyclic(&rir, &interner));

    Ok(CodegenResult {
        ll,
        const_print_i64,
    })
}

/// Concatenate dependency entries (first) then primary into one parse unit.
fn build_unity_source(
    primary: &Path,
    dep_entries: &[std::path::PathBuf],
) -> Result<(String, String), String> {
    let mut unity = String::new();
    for dep in dep_entries {
        let text = std::fs::read_to_string(dep)
            .map_err(|e| format!("cannot read dependency {}: {e}", dep.display()))?;
        if has_def_main(&text) {
            return Err(format!(
                "dependency entry {} defines `main` — library packages must not",
                dep.display()
            ));
        }
        unity.push_str(&format!("## package unit: {}\n", dep.display()));
        unity.push_str(&text);
        if !text.ends_with('\n') {
            unity.push('\n');
        }
        unity.push('\n');
    }
    let primary_text = std::fs::read_to_string(primary)
        .map_err(|e| format!("cannot read {}: {e}", primary.display()))?;
    if !dep_entries.is_empty() {
        unity.push_str(&format!("## package unit: {}\n", primary.display()));
    }
    unity.push_str(&primary_text);
    if !primary_text.ends_with('\n') {
        unity.push('\n');
    }
    let name = if dep_entries.is_empty() {
        primary.display().to_string()
    } else {
        format!("unity:{}", primary.display())
    };
    Ok((name, unity))
}

fn has_def_main(src: &str) -> bool {
    for line in src.lines() {
        let t = line.split("##").next().unwrap_or("").trim();
        if t.starts_with("def main") {
            return true;
        }
    }
    false
}

/// Folded / unrolled kernels with no back-edges: interpret once at compile time.
fn eval_const_print_if_acyclic(
    module: &Module,
    interner: &rynix_span::Interner,
) -> Option<i64> {
    if module_has_back_edge(module) {
        return None;
    }
    match interpret_module_print(module, interner) {
        Ok((_, Some(n))) => Some(n),
        _ => None,
    }
}

fn module_has_back_edge(module: &Module) -> bool {
    for func in &module.funcs {
        for (bi, block) in func.blocks.iter().enumerate() {
            let Some(&term) = block.insts.last() else {
                continue;
            };
            match func.inst(term) {
                Inst::Jump { target, .. } if target.0 as usize <= bi => return true,
                Inst::Br {
                    then_target,
                    else_target,
                    ..
                } if then_target.0 as usize <= bi || else_target.0 as usize <= bi => {
                    return true;
                }
                _ => {}
            }
        }
    }
    false
}

/// `main` prints one i64 — either a literal or `%t = add i64 0, LIT` / iconst materialization.
fn detect_const_print_i64(ll: &str) -> Option<i64> {
    if ll.contains(" phi ") || ll.contains("urem ") || ll.contains("srem ") {
        return None;
    }
    // Map %tN → constant for `add i64 0, N` / `add i64 N, 0` style iconsts.
    let mut consts: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for line in ll.lines() {
        let line = line.trim();
        let Some((lhs, rhs)) = line.split_once('=') else {
            continue;
        };
        let name = lhs.trim();
        if !name.starts_with('%') {
            continue;
        }
        let rhs = rhs.trim();
        if let Some(rest) = rhs.strip_prefix("add i64 0, ") {
            if let Ok(n) = rest.trim().parse::<i64>() {
                consts.insert(name, n);
            }
        } else if let Some(rest) = rhs.strip_prefix("add i64 ") {
            if let Some((n, z)) = rest.split_once(", 0") {
                if z.is_empty() {
                    if let Ok(n) = n.trim().parse::<i64>() {
                        consts.insert(name, n);
                    }
                }
            }
        }
    }
    let mut found = None;
    for line in ll.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("call void @rynix_rt_print_i64(i64 ") else {
            continue;
        };
        let arg = rest.strip_suffix(')')?;
        let n = if let Ok(lit) = arg.parse::<i64>() {
            lit
        } else {
            *consts.get(arg)?
        };
        if found.is_some() {
            return None;
        }
        found = Some(n);
    }
    found
}

/// Locate the `rt/` directory (contains `portable.c` and `include/`).
pub fn runtime_root() -> Option<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for c in [
            dir.join("rt"),
            dir.join("../rt"),
            dir.join("../../rt"),
        ] {
            if c.join("portable.c").is_file() {
                return Some(c);
            }
        }
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in ["../../rt", "../rt"] {
        let ws = manifest.join(rel);
        if ws.join("portable.c").is_file() {
            return Some(ws.canonicalize().unwrap_or(ws));
        }
    }
    None
}

/// Locate `rt/portable.c` (unity build of the portable runtime).
#[allow(dead_code)]
pub fn portable_runtime_c() -> Option<std::path::PathBuf> {
    runtime_root().map(|r| r.join("portable.c"))
}
