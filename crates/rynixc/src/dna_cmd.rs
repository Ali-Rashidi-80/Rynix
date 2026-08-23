//! `rynixc dna` — project conventions report (`rynix.dna.v1`).

use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{DnaOptions, ErrorFormat};
use crate::dna::mine_dna;

pub fn run(options: &DnaOptions) -> ExitCode {
    let root = options
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let report = mine_dna(&root);
    match options.error_format {
        ErrorFormat::Json => println!("{}", report.to_json()),
        ErrorFormat::Human => {
            if options.prompt {
                print!("{}", report.to_prompt());
            } else {
                println!("project: {}", report.project_name);
                println!("  files:      {}", report.scanned_files);
                println!("  defs:       {}", report.scanned_defs);
                println!("  functions:  {}", report.function_style());
                println!("  structs:    {}", report.struct_style());
                println!("  memory:     {}", report.memory_strategy());
                println!("  concurrency:{}", report.concurrency_model());
                println!(
                    "  arch.toml:  {}  rynix.toml: {}",
                    report.architecture_toml, report.rynix_toml
                );
                println!(
                    "  signals:    region={} pipe={} fibers={} http={} tls={} pure={}",
                    report.uses_region,
                    report.uses_pipe,
                    report.uses_fibers,
                    report.uses_http,
                    report.uses_tls,
                    report.effect_pure_attrs
                );
                println!("  confidence: {:.0}%", report.confidence * 100.0);
                println!("  (heuristic — see rynix.dna.v1)");
            }
        }
    }
    ExitCode::SUCCESS
}
