//! `rynixc scope` — show agent permission profile (rynix.scope.v1).

use std::process::ExitCode;

use crate::cli::{ErrorFormat, ScopeOptions};
use crate::scope::load_scope;

pub fn run(options: &ScopeOptions) -> ExitCode {
    let cfg = load_scope(options.config.as_deref());
    match options.error_format {
        ErrorFormat::Human => {
            println!(
                "scope: patch_write={} (from {})",
                cfg.patch_write, cfg.source
            );
            if !cfg.patch_write {
                println!("  patch --write is denied unless rynix.scope.toml sets patch_write=true");
                println!("  or you pass --force-write (explicit override).");
            }
        }
        ErrorFormat::Json => println!("{}", cfg.to_json()),
    }
    ExitCode::SUCCESS
}
