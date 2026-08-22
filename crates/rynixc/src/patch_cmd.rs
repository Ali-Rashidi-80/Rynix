//! `rynixc patch` — apply the best compiler-suggested fix to a source file.

use std::fs;
use std::process::ExitCode;

use crate::cli::PatchOptions;
use crate::fix::apply_first_fix;

pub fn run(options: &PatchOptions) -> ExitCode {
    let src = match fs::read_to_string(&options.path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", options.path.display());
            return ExitCode::from(3);
        }
    };
    let fixed = apply_first_fix(&src);
    if options.write {
        if let Err(e) = fs::write(&options.path, &fixed) {
            eprintln!("error: cannot write {}: {e}", options.path.display());
            return ExitCode::from(3);
        }
        eprintln!("patched {}", options.path.display());
    } else {
        print!("{fixed}");
    }
    ExitCode::SUCCESS
}
