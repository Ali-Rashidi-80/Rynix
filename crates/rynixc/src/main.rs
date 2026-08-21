//! The Rynix compiler driver.
//!
//! ```text
//! rynixc lex      <file.ryx> [--dump-tokens] [--error-format=human|json]
//! rynixc parse    <file.ryx> [--dump-ast]    [--error-format=human|json]
//! rynixc check    <file.ryx>                 [--error-format=human|json]
//! rynixc dump-rir <file.ryx> [--opt]         [--error-format=human|json]
//! ```

mod check_cmd;
mod cli;
mod driver;
mod dump_rir_cmd;
mod lex_cmd;
mod parse_cmd;

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
        Ok(cli::Command::Parse(options)) => parse_cmd::run(&options),
        Ok(cli::Command::Check(options)) => check_cmd::run(&options),
        Ok(cli::Command::DumpRir(options)) => dump_rir_cmd::run(&options),
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}
