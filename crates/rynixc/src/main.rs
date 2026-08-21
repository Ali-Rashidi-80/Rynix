//! The Rynix compiler driver.
//!
//! Phase 1 exposes a single subcommand, `lex`, which is enough to exercise
//! the whole front-end plumbing: memory-mapped source loading, the
//! zero-allocation lexer, and both diagnostic renderers.
//!
//! ```text
//! rynixc lex <file.ryx> [--dump-tokens] [--error-format=human|json]
//! ```

mod cli;
mod lex_cmd;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Version) => {
            println!("rynixc {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Lex(options)) => lex_cmd::run(&options),
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}
