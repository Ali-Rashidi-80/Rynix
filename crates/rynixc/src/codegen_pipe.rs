//! Shared front-end → RIR pipeline for codegen subcommands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// One unity-compile unit: package name (mangling prefix) + entry path.
#[derive(Debug, Clone)]
pub struct CompileUnit {
    pub name: String,
    pub path: PathBuf,
}

/// Compile one primary `.ryx` (no package deps).
#[allow(dead_code)]
pub fn compile_to_llvm(
    path: &Path,
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    compile_to_llvm_with_units(path, &[], optimize, error_format)
}

/// Unity-compile primary + dependency/std units (SPEC §6.3–6.5).
///
/// Dependency `def` names are mangled to `pkg__fn`. Soft builtins stay unmangled.
#[allow(dead_code)] // legacy PathBuf helper
pub fn compile_to_llvm_with_deps(
    primary: &Path,
    dep_entries: &[PathBuf],
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    let units: Vec<CompileUnit> = dep_entries
        .iter()
        .map(|p| CompileUnit {
            name: package_name_from_entry(p),
            path: p.clone(),
        })
        .collect();
    compile_to_llvm_with_units(primary, &units, optimize, error_format)
}

pub fn compile_to_llvm_with_units(
    primary: &Path,
    dep_units: &[CompileUnit],
    optimize: bool,
    error_format: ErrorFormat,
) -> Result<CodegenResult, ExitCode> {
    let (unity_name, unity_text) = match build_unity_source(primary, dep_units) {
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
    let const_print_i64 =
        detect_const_print_i64(&ll).or_else(|| eval_const_print_if_acyclic(&rir, &interner));

    Ok(CodegenResult {
        ll,
        const_print_i64,
    })
}

#[allow(dead_code)]
fn package_name_from_entry(entry: &Path) -> String {
    entry
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dep".into())
}

/// Concatenate mangled dependency units, std imports, then primary.
fn build_unity_source(
    primary: &Path,
    dep_units: &[CompileUnit],
) -> Result<(String, String), String> {
    let mut unity = String::new();
    let mut exports: HashMap<String, String> = HashMap::new();
    let mut pkg_prefixes: HashMap<String, String> = HashMap::new();

    for unit in dep_units {
        let text = std::fs::read_to_string(&unit.path)
            .map_err(|e| format!("cannot read dependency {}: {e}", unit.path.display()))?;
        if has_def_main(&text) {
            return Err(format!(
                "dependency entry {} defines `main` — library packages must not",
                unit.path.display()
            ));
        }
        let prefix = sanitize_pkg_prefix(&unit.name);
        pkg_prefixes.insert(unit.name.clone(), prefix.clone());
        let mangled = mangle_unit(&prefix, &text, &mut exports)?;
        unity.push_str(&format!(
            "## package unit: {} ({prefix}__*)\n",
            unit.path.display()
        ));
        unity.push_str(&mangled);
        if !mangled.ends_with('\n') {
            unity.push('\n');
        }
        unity.push('\n');
    }

    let primary_text = std::fs::read_to_string(primary)
        .map_err(|e| format!("cannot read {}: {e}", primary.display()))?;

    // `import std.math` → load std/math.ryx (real defs only; soft builtins stay in sema).
    let std_units = collect_std_imports(&primary_text)?;
    for (mod_name, path) in &std_units {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read std module {}: {e}", path.display()))?;
        if has_def_main(&text) {
            return Err(format!("std module {} must not define main", path.display()));
        }
        let prefix = sanitize_pkg_prefix(mod_name);
        pkg_prefixes.insert(mod_name.clone(), prefix.clone());
        let mangled = mangle_unit(&prefix, &text, &mut exports)?;
        unity.push_str(&format!("## std unit: {} ({prefix}__*)\n", path.display()));
        unity.push_str(&mangled);
        if !mangled.ends_with('\n') {
            unity.push('\n');
        }
        unity.push('\n');
    }

    let mut app = primary_text.clone();
    // Rewrite bare calls to unique dep/std exports.
    let mut pairs: Vec<_> = exports.iter().collect();
    pairs.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    for (bare, mangled) in pairs {
        app = rewrite_call_name(&app, bare, mangled);
    }
    // Rewrite `pkg.fn(` → `pkg__fn(` for method-style package calls.
    for (pkg, prefix) in &pkg_prefixes {
        app = rewrite_qualified_calls(&app, pkg, prefix);
    }

    if !dep_units.is_empty() || !std_units.is_empty() {
        unity.push_str(&format!("## package unit: {}\n", primary.display()));
    }
    unity.push_str(&app);
    if !app.ends_with('\n') {
        unity.push('\n');
    }
    let name = if dep_units.is_empty() && std_units.is_empty() {
        primary.display().to_string()
    } else {
        format!("unity:{}", primary.display())
    };
    Ok((name, unity))
}

fn sanitize_pkg_prefix(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "pkg".into()
    } else if out.as_bytes()[0].is_ascii_digit() {
        format!("p_{out}")
    } else {
        out
    }
}

fn collect_def_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let t = line.split("##").next().unwrap_or("").trim();
        let Some(rest) = t.strip_prefix("def ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

fn mangle_unit(
    prefix: &str,
    text: &str,
    exports: &mut HashMap<String, String>,
) -> Result<String, String> {
    let mut out = text.to_string();
    // Rewrite calls to already-exported symbols (transitive deps).
    let mut prior: Vec<_> = exports.iter().map(|(a, b)| (a.clone(), b.clone())).collect();
    prior.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    for (bare, mangled) in &prior {
        out = rewrite_call_name(&out, bare, mangled);
    }

    let defs = collect_def_names(&out);
    for name in &defs {
        let mangled = format!("{prefix}__{name}");
        if let Some(prev) = exports.get(name) {
            if prev != &mangled {
                return Err(format!(
                    "duplicate def `{name}` across packages (`{prev}` vs `{mangled}`)"
                ));
            }
        } else {
            exports.insert(name.clone(), mangled);
        }
    }

    let mut defs_sorted = defs;
    defs_sorted.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for name in defs_sorted {
        let mangled = format!("{prefix}__{name}");
        out = rewrite_def_name(&out, &name, &mangled);
        out = rewrite_call_name(&out, &name, &mangled);
    }
    Ok(out)
}

fn rewrite_def_name(text: &str, from: &str, to: &str) -> String {
    let needle = format!("def {from}");
    let repl = format!("def {to}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(&needle) {
        let after = i + needle.len();
        let ok_boundary = rest
            .as_bytes()
            .get(after)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        out.push_str(&rest[..i]);
        if ok_boundary {
            out.push_str(&repl);
            rest = &rest[after..];
        } else {
            out.push_str(&needle);
            rest = &rest[after..];
        }
    }
    out.push_str(rest);
    out
}

fn rewrite_call_name(text: &str, from: &str, to: &str) -> String {
    let needle = format!("{from}(");
    let repl = format!("{to}(");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(&needle) {
        let before_ok = if i == 0 {
            true
        } else {
            let b = rest.as_bytes()[i - 1];
            !b.is_ascii_alphanumeric() && b != b'_' && b != b'.'
        };
        out.push_str(&rest[..i]);
        if before_ok {
            out.push_str(&repl);
            rest = &rest[i + needle.len()..];
        } else {
            out.push_str(&needle);
            rest = &rest[i + needle.len()..];
        }
    }
    out.push_str(rest);
    out
}

/// `util.foo(` → `util__foo(` so method-call sugar becomes a bare mangled call.
fn rewrite_qualified_calls(text: &str, pkg: &str, prefix: &str) -> String {
    let needle = format!("{pkg}.");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(&needle) {
        let after = i + needle.len();
        let before_ok = if i == 0 {
            true
        } else {
            let b = rest.as_bytes()[i - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        out.push_str(&rest[..i]);
        if !before_ok {
            out.push_str(&needle);
            rest = &rest[after..];
            continue;
        }
        let rem = &rest[after..];
        let name: String = rem
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() || !rem[name.len()..].starts_with('(') {
            out.push_str(&needle);
            rest = &rest[after..];
            continue;
        }
        out.push_str(prefix);
        out.push_str("__");
        out.push_str(&name);
        out.push('(');
        rest = &rem[name.len() + 1..];
    }
    out.push_str(rest);
    out
}

fn collect_std_imports(primary_text: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let Some(std_root) = std_root() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in primary_text.lines() {
        let t = line.split("##").next().unwrap_or("").trim();
        let Some(rest) = t.strip_prefix("import ") else {
            continue;
        };
        let path = rest.trim();
        // `std::math` (SPEC path) or legacy `std.math`
        let norm = path.replace('.', "::");
        let mut parts = norm.split("::").map(str::trim).filter(|s| !s.is_empty());
        let Some(first) = parts.next() else {
            continue;
        };
        if first != "std" {
            continue;
        }
        let module = parts.next().unwrap_or("core");
        if parts.next().is_some() {
            return Err(format!(
                "import `{path}`: only `import std` or `import std::<module>` supported"
            ));
        }
        if !seen.insert(module.to_string()) {
            continue;
        }
        let file = std_root.join(format!("{module}.ryx"));
        if !file.is_file() {
            return Err(format!(
                "std module `{module}` not found at {}",
                file.display()
            ));
        }
        // Skip docs-only modules with no `def` (soft builtins cover those).
        let text = std::fs::read_to_string(&file)
            .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        if collect_def_names(&text).is_empty() {
            continue;
        }
        out.push((module.to_string(), file));
    }
    Ok(out)
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

fn std_root() -> Option<PathBuf> {
    runtime_root().map(|rt| rt.parent().unwrap_or(&rt).join("std"))
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
pub fn runtime_root() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for c in [dir.join("rt"), dir.join("../rt"), dir.join("../../rt")] {
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
pub fn portable_runtime_c() -> Option<PathBuf> {
    runtime_root().map(|r| r.join("portable.c"))
}
