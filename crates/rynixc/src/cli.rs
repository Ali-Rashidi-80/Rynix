//! Argument parsing.
//!
//! Hand-written on purpose: the driver has one canonical invocation form per
//! task, so a parser generator would add a dependency and a second way to
//! express everything.

use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: rynixc <command> [options]

Commands:
  lex <file.ryx>      Tokenize a source file
  parse <file.ryx>    Parse a source file into an arena AST
  check <file.ryx>    Lex + parse + sema; report diagnostics
  dump-rir <file.ryx> Lower to RIR and print textual form

Options for `lex`:
  --dump-tokens       Print one line per token: span, kind, text

Options for `parse`:
  --dump-ast          Print the AST as an s-expression

Options for `check`:
  --explain-alloc     After a clean check, print allocation placement
                      (stack|region|heap) for every RIR alloc site

Options for `dump-rir`:
  --opt               Run DCE / const-fold / simplify-cfg before dump
  --escape            Run escape analysis and inject region/free markers

Shared options:
  --error-format=FMT  Diagnostic rendering: `human` (default) or `json`
                      (`json` emits one rynix.diag.v1 object per line;
                       schema: docs/schemas/rynix.diag.v1.json)

Global options:
  -h, --help          Print this help
  -V, --version       Print the compiler version

Exit codes: 0 success, 1 diagnostics reported, 2 bad invocation, 3 I/O error
";

/// How diagnostics are rendered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorFormat {
    Human,
    Json,
}

#[derive(Debug)]
pub struct LexOptions {
    pub path: PathBuf,
    pub dump_tokens: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct ParseOptions {
    pub path: PathBuf,
    pub dump_ast: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub struct CheckOptions {
    pub path: PathBuf,
    pub error_format: ErrorFormat,
    pub explain_alloc: bool,
}

#[derive(Debug)]
pub struct DumpRirOptions {
    pub path: PathBuf,
    pub optimize: bool,
    pub escape: bool,
    pub error_format: ErrorFormat,
}

#[derive(Debug)]
pub enum Command {
    Help,
    Version,
    Lex(LexOptions),
    Parse(ParseOptions),
    Check(CheckOptions),
    DumpRir(DumpRirOptions),
}

/// Parses command-line arguments (without the program name).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(first) = args.first() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" | "version" => Ok(Command::Version),
        "lex" => parse_lex(&args[1..]),
        "parse" => parse_parse(&args[1..]),
        "check" => parse_check(&args[1..]),
        "dump-rir" => parse_dump_rir(&args[1..]),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn parse_error_format(arg: &str) -> Result<ErrorFormat, String> {
    match arg {
        "--error-format=human" => Ok(ErrorFormat::Human),
        "--error-format=json" => Ok(ErrorFormat::Json),
        other if other.starts_with("--error-format") => Err(format!(
            "invalid `{other}`: expected --error-format=human or --error-format=json"
        )),
        _ => unreachable!(),
    }
}

fn parse_lex(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut dump_tokens = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--dump-tokens" => dump_tokens = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Lex(LexOptions {
        path,
        dump_tokens,
        error_format,
    }))
}

fn parse_parse(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut dump_ast = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--dump-ast" => dump_ast = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Parse(ParseOptions {
        path,
        dump_ast,
        error_format,
    }))
}

fn parse_check(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut error_format = ErrorFormat::Human;
    let mut explain_alloc = false;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--explain-alloc" => explain_alloc = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Check(CheckOptions {
        path,
        error_format,
        explain_alloc,
    }))
}

fn parse_dump_rir(args: &[String]) -> Result<Command, String> {
    let mut path = None;
    let mut optimize = false;
    let mut escape = false;
    let mut error_format = ErrorFormat::Human;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--opt" => optimize = true,
            "--escape" => escape = true,
            other if other.starts_with("--error-format") => {
                error_format = parse_error_format(other)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"));
            }
            other if path.is_some() => {
                return Err(format!("unexpected extra argument `{other}`"));
            }
            other => path = Some(PathBuf::from(other)),
        }
    }

    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::DumpRir(DumpRirOptions {
        path,
        optimize,
        escape,
        error_format,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_arguments_prints_help() {
        assert!(matches!(parse(&[]).unwrap(), Command::Help));
        assert!(matches!(parse(&args(&["--help"])).unwrap(), Command::Help));
        assert!(matches!(parse(&args(&["-V"])).unwrap(), Command::Version));
    }

    #[test]
    fn lex_defaults_to_human_errors_without_token_dump() {
        let Command::Lex(options) = parse(&args(&["lex", "a.ryx"])).unwrap() else {
            panic!("expected lex");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(!options.dump_tokens);
        assert_eq!(options.error_format, ErrorFormat::Human);
    }

    #[test]
    fn lex_accepts_all_flags_in_any_order() {
        let Command::Lex(options) = parse(&args(&[
            "lex",
            "--error-format=json",
            "b.ryx",
            "--dump-tokens",
        ]))
        .unwrap() else {
            panic!("expected lex");
        };
        assert_eq!(options.path.to_str(), Some("b.ryx"));
        assert!(options.dump_tokens);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn parse_accepts_dump_ast() {
        let Command::Parse(options) = parse(&args(&[
            "parse",
            "a.ryx",
            "--dump-ast",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected parse");
        };
        assert!(options.dump_ast);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn dump_rir_prints_func() {
        let Command::DumpRir(options) = parse(&args(&[
            "dump-rir",
            "a.ryx",
            "--opt",
            "--escape",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected dump-rir");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(options.optimize);
        assert!(options.escape);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn check_accepts_json_and_explain() {
        let Command::Check(options) = parse(&args(&[
            "check",
            "a.ryx",
            "--explain-alloc",
            "--error-format=json",
        ]))
        .unwrap() else {
            panic!("expected check");
        };
        assert_eq!(options.path.to_str(), Some("a.ryx"));
        assert!(options.explain_alloc);
        assert_eq!(options.error_format, ErrorFormat::Json);
    }

    #[test]
    fn invocation_errors_are_specific() {
        assert!(
            parse(&args(&["lex"]))
                .unwrap_err()
                .contains("missing input")
        );
        assert!(
            parse(&args(&["build"]))
                .unwrap_err()
                .contains("unknown command")
        );
        assert!(
            parse(&args(&["lex", "a", "--nope"]))
                .unwrap_err()
                .contains("unknown option")
        );
        assert!(
            parse(&args(&["lex", "a", "b"]))
                .unwrap_err()
                .contains("extra argument")
        );
        assert!(
            parse(&args(&["lex", "a", "--error-format=xml"]))
                .unwrap_err()
                .contains("expected --error-format")
        );
    }
}
