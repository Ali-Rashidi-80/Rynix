//! `rynixc security <file.ryx>` — pattern-based CWE-798 class scan.

use std::fs;
use std::process::ExitCode;

use crate::cli::{ErrorFormat, SecurityOptions};
use crate::security::scan_source;

pub fn run(options: &SecurityOptions) -> ExitCode {
    let src = match fs::read_to_string(&options.path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", options.path.display());
            return ExitCode::from(3);
        }
    };
    let path = options.path.display().to_string();
    let report = scan_source(&path, &src);

    match options.error_format {
        ErrorFormat::Human => {
            if report.findings.is_empty() {
                println!("security: no pattern findings in {path}");
            } else {
                eprintln!(
                    "security: {} finding(s) in {path} (pattern scan; not a full audit)",
                    report.findings.len()
                );
                for f in &report.findings {
                    eprintln!(
                        "  {}:{} [{}] {} — {}",
                        path, f.line, f.severity, f.cwe, f.title
                    );
                    eprintln!("    {}", f.snippet);
                }
            }
        }
        ErrorFormat::Json => println!("{}", report.to_json()),
    }

    if report.blocking() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
