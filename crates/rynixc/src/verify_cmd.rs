//! `rynixc verify --contract=PATH.toml` — evidence-gated agent contracts.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{ErrorFormat, VerifyOptions};
use crate::contract::ContractEngine;

pub fn run(options: &VerifyOptions) -> ExitCode {
    let contract = match ContractEngine::load(&options.contract) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let root = options.root.clone().unwrap_or_else(|| PathBuf::from("."));
    let report = ContractEngine::verify(&contract, &root, options.run_tests);

    match options.error_format {
        ErrorFormat::Human => {
            if report.all_ok() {
                println!(
                    "verify passed (contract `{}`, {} evidence, ran_tests={})",
                    report.contract,
                    report.evidence.len(),
                    report.ran_tests
                );
                for e in &report.evidence {
                    println!("  ok  {}: {}", e.id, e.detail);
                }
            } else {
                eprintln!(
                    "verify failed (contract `{}`):",
                    report.contract
                );
                for e in &report.evidence {
                    let mark = if e.ok { "ok " } else { "FAIL" };
                    eprintln!("  {mark} {}: {}", e.id, e.detail);
                }
            }
        }
        ErrorFormat::Json => {
            println!("{}", report.to_json());
        }
    }

    if report.all_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
