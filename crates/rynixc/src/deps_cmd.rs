//! `rynixc deps` — resolve local path dependencies from `rynix.toml`.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{DepsOptions, ErrorFormat};
use crate::manifest::{find_manifest, load_manifest, resolve_deps};

pub fn run(options: &DepsOptions) -> ExitCode {
    let start = options
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let Some(m_path) = find_manifest(&start) else {
        eprintln!(
            "error: no rynix.toml found from {}",
            start.display()
        );
        return ExitCode::from(1);
    };
    let manifest = match load_manifest(&m_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let report = resolve_deps(&manifest);
    match options.error_format {
        ErrorFormat::Json => {
            println!("{}", report.to_json());
        }
        ErrorFormat::Human => {
            println!(
                "package `{}` ({}): {} path dep(s)",
                report.package,
                report.root_manifest.display(),
                report.deps.len()
            );
            for d in &report.deps {
                let mark = if d.ok { "ok" } else { "FAIL" };
                println!(
                    "  [{mark}] {} -> {} ({})",
                    d.name,
                    d.path.display(),
                    d.detail
                );
            }
            if report.deps.is_empty() {
                println!("  (no [dependencies] — local path deps only; no registry)");
            }
        }
    }
    if report.all_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
