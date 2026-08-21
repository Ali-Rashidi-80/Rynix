//! `rynixc test` — run `#^` directive corpora under testdata/ (and optional paths).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};

use crate::cli::TestOptions;

pub fn run(options: &TestOptions) -> ExitCode {
    let roots: Vec<PathBuf> = if options.paths.is_empty() {
        vec![PathBuf::from("testdata")]
    } else {
        options.paths.clone()
    };

    let mut files = Vec::new();
    for root in &roots {
        collect_ryx(root, &mut files);
    }
    if files.is_empty() {
        eprintln!("error: no .ryx files found");
        return ExitCode::from(1);
    }

    let mut failed = 0usize;
    let mut ran = 0usize;
    for path in &files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: read error: {e}", path.display());
                failed += 1;
                continue;
            }
        };
        if !src.contains("#^") {
            continue;
        }
        ran += 1;
        if let Err(msg) = check_directives(path, &src) {
            eprintln!("FAIL {}: {msg}", path.display());
            failed += 1;
        } else {
            println!("ok {}", path.display());
        }
    }

    if ran == 0 {
        println!("no #^ directive tests found (checked {} files)", files.len());
        return ExitCode::SUCCESS;
    }
    if failed > 0 {
        eprintln!("{failed}/{ran} directive test(s) failed");
        ExitCode::from(1)
    } else {
        println!("{ran} directive test(s) passed");
        ExitCode::SUCCESS
    }
}

fn collect_ryx(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(meta) = std::fs::metadata(root) else {
        return;
    };
    if meta.is_file() {
        if root.extension().and_then(|e| e.to_str()) == Some("ryx") {
            out.push(root.to_path_buf());
        }
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    for ent in rd.flatten() {
        collect_ryx(&ent.path(), out);
    }
}

fn check_directives(path: &Path, src: &str) -> Result<(), String> {
    let mut expects = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if let Some(idx) = line.find("#^") {
            let rest = line[idx + 2..].trim_start();
            if let Some(rest) = rest.strip_prefix("error") {
                let code = rest.split_whitespace().next().unwrap_or("");
                if code.starts_with("RYX") {
                    expects.push(((i + 1) as u32, code.to_string()));
                }
            }
        }
    }
    if expects.is_empty() {
        return Ok(());
    }

    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, src, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);

    let mut sm = SourceMap::new();
    sm.add_owned(path.to_string_lossy(), src.to_string());

    let mut unmatched: std::collections::HashSet<(u32, String)> =
        expects.into_iter().collect();
    for diag in &sink.diags {
        let (_, lc) = sm.line_col(diag.primary.span.lo());
        unmatched.remove(&(lc.line, diag.code.as_str().to_string()));
    }
    if unmatched.is_empty() {
        Ok(())
    } else {
        Err(format!("unmet directives: {unmatched:?}"))
    }
}
