//! `rynixc arch check` — validate Architecture.toml layer boundaries.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::architecture::ArchitectureEngine;
use crate::cli::{ArchCheckOptions, ErrorFormat};

pub fn run(options: &ArchCheckOptions) -> ExitCode {
    let config = match ArchitectureEngine::load_config(options.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let root = options.root.clone().unwrap_or_else(|| PathBuf::from("."));
    let report = ArchitectureEngine::check_project(&config, &root);

    match options.error_format {
        ErrorFormat::Human => {
            if report.violations.is_empty() {
                println!(
                    "architecture check passed ({} rules, {} .ryx files)",
                    report.rules_checked, report.files_scanned
                );
            } else {
                eprintln!(
                    "architecture check failed: {} violation(s)\n",
                    report.violations_count
                );
                for v in &report.violations {
                    eprintln!(
                        "  {}:{} [{}] {}",
                        v.file, v.line, v.rule_pattern, v.message
                    );
                }
            }
        }
        ErrorFormat::Json => {
            println!("{}", report.to_json());
        }
    }

    if report.violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
