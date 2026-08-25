//! `rynixc deps` — resolve local path dependencies from `rynix.toml`.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::attest::{
    attest_path_for_manifest, enrich_attest_json, from_lock, read_attest, verify_attest,
    write_attest,
};
use crate::cli::{DepsOptions, ErrorFormat};
use crate::lockfile::{
    enrich_deps_json, lock_from_report, lock_path_for_manifest, read_lock, verify_report, write_lock,
};
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
            let mut body = enrich_deps_json(&report, report.to_json());
            body = enrich_attest_json(&report, body);
            println!("{body}");
        }
        ErrorFormat::Human => {
            println!(
                "package `{}` ({}): {} dep(s)",
                report.package,
                report.root_manifest.display(),
                report.deps.len()
            );
            if let Some(reg) = &report.registry {
                println!("  registry: {}", reg.display());
            }
            if let Some(idx) = &report.registry_index {
                println!("  registry_index: {idx}");
            }
            for d in &report.deps {
                let mark = if d.ok { "ok" } else { "FAIL" };
                let ver = d
                    .version
                    .as_ref()
                    .map(|v| format!(" @{v}"))
                    .unwrap_or_default();
                let nsrc = if d.sources.is_empty() {
                    String::new()
                } else {
                    format!(" [{} src]", d.sources.len())
                };
                println!(
                    "  [{mark}] {} ({}){}{} -> {} ({})",
                    d.name,
                    d.kind,
                    ver,
                    nsrc,
                    d.path.display(),
                    d.detail
                );
            }
            if report.deps.is_empty() {
                println!("  (no [dependencies])");
            }
        }
    }
    if !report.all_ok() {
        return ExitCode::from(1);
    }

    let lock_path = lock_path_for_manifest(&report.root_manifest);
    if options.locked {
        if !lock_path.is_file() {
            eprintln!(
                "error: --locked requires {} (run `rynixc deps --lock` first)",
                lock_path.display()
            );
            return ExitCode::from(1);
        }
        match read_lock(&lock_path).and_then(|lock| verify_report(&report, &lock)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        }
    } else if lock_path.is_file() {
        if let Err(e) = read_lock(&lock_path).and_then(|lock| verify_report(&report, &lock)) {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    }

    let write_lockfile = options.lock || options.attest;
    if write_lockfile {
        match lock_from_report(&report).and_then(|lock| write_lock(&lock_path, &lock)) {
            Ok(()) => {
                if matches!(options.error_format, ErrorFormat::Human) {
                    eprintln!("ok: wrote {}", lock_path.display());
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        }
    }

    if options.attest {
        let lock = match read_lock(&lock_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        };
        let attest_path = attest_path_for_manifest(&report.root_manifest);
        match from_lock(&lock_path, &lock).and_then(|a| write_attest(&attest_path, &a)) {
            Ok(()) => {
                if matches!(options.error_format, ErrorFormat::Human) {
                    eprintln!(
                        "ok: wrote {} (schema rynix.attest.v1, local digest — not Sigstore/Rekor)",
                        attest_path.display()
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        }
    }

    if options.attest_verify {
        let attest_path = attest_path_for_manifest(&report.root_manifest);
        if !attest_path.is_file() {
            eprintln!(
                "error: --attest-verify requires {} (run `rynixc deps --attest` first)",
                attest_path.display()
            );
            return ExitCode::from(1);
        }
        if !lock_path.is_file() {
            eprintln!(
                "error: --attest-verify requires {} (run `rynixc deps --lock` first)",
                lock_path.display()
            );
            return ExitCode::from(1);
        }
        match (read_lock(&lock_path), read_attest(&attest_path)) {
            (Ok(lock), Ok(attest)) => {
                if let Err(e) = verify_attest(&report, &lock_path, &lock, &attest) {
                    eprintln!("error: {e}");
                    return ExitCode::from(1);
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("error: {e}");
                return ExitCode::from(1);
            }
        }
    }

    ExitCode::SUCCESS
}
