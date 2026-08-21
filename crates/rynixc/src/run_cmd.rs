//! `rynixc run` — build then execute.

use std::process::{Command, ExitCode};

use crate::build_cmd;
use crate::cli::{BuildOptions, RunOptions};

pub fn run(options: &RunOptions) -> ExitCode {
    let out = options
        .output
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("rynix_run_bin"));

    let build = BuildOptions {
        path: options.path.clone(),
        output: Some(out.clone()),
        keep_ll: false,
        runtime: options.runtime,
        error_format: options.error_format,
    };
    let code = build_cmd::run(&build);
    if code != ExitCode::SUCCESS {
        return code;
    }

    let exe = if cfg!(windows) && out.extension().is_none() {
        out.with_extension("exe")
    } else {
        out
    };
    // MinGW clang may write without .exe when -o has no extension — try both.
    let exe = if exe.is_file() {
        exe
    } else if exe.with_extension("exe").is_file() {
        exe.with_extension("exe")
    } else {
        eprintln!("error: built binary not found at {}", exe.display());
        return ExitCode::from(1);
    };

    match Command::new(&exe).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            ExitCode::from(u8::try_from(code).unwrap_or(1))
        }
        Err(e) => {
            eprintln!("error: failed to run {}: {e}", exe.display());
            ExitCode::from(1)
        }
    }
}
