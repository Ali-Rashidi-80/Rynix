//! End-to-end tests for `rynixc lex`, driving the real binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn rynixc() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so the driver is two
    // directories up.
    let mut path = std::env::current_exe().expect("test exe path");
    path.pop();
    path.pop();
    path.push(format!("rynixc{}", std::env::consts::EXE_SUFFIX));
    assert!(path.is_file(), "driver not built at {}", path.display());
    path
}

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/lexer")
        .join(name)
        .canonicalize()
        .expect("corpus file exists")
}

fn run(args: &[&str]) -> Output {
    Command::new(rynixc())
        .args(args)
        .output()
        .expect("spawn rynixc")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

#[test]
fn clean_file_succeeds_and_dumps_tokens() {
    let path = corpus("hello.ryx");
    let output = run(&[path.to_str().unwrap(), "--dump-tokens"]);
    // Note: the subcommand is `lex`; without it we expect an invocation error.
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));

    let output = run(&["lex", path.to_str().unwrap(), "--dump-tokens"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let dump = stdout(&output);
    assert!(dump.contains("DocComment"), "{dump}");
    assert!(dump.contains("Def"), "{dump}");
    assert!(dump.contains("StrLit"), "{dump}");
    // The last line is the Eof token; compare fields, not column widths.
    let last: Vec<&str> = dump
        .lines()
        .rev()
        .find(|l| !l.is_empty())
        .unwrap()
        .split_whitespace()
        .collect();
    assert_eq!(last[1], "Eof", "{dump}");
    assert_eq!(last[2], "\"\"", "{dump}");
    assert!(stderr(&output).is_empty(), "no diagnostics expected");
}

#[test]
fn without_dump_tokens_stdout_is_empty() {
    let path = corpus("hello.ryx");
    let output = run(&["lex", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).is_empty());
}

#[test]
fn broken_file_reports_diagnostics_and_exits_one() {
    let path = corpus("errors.ryx");
    let output = run(&["lex", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    for code in ["RYX0001", "RYX0002", "RYX0003", "RYX0004", "RYX0005"] {
        assert!(text.contains(code), "missing {code} in:\n{text}");
    }
    assert!(text.contains("errors reported"), "{text}");
}

#[test]
fn json_format_emits_one_object_per_line() {
    let path = corpus("errors.ryx");
    let output = run(&["lex", path.to_str().unwrap(), "--error-format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() > 10,
        "expected many diagnostics, got {}",
        lines.len()
    );
    for line in &lines {
        assert!(
            line.starts_with('{') && line.ends_with('}'),
            "not NDJSON: {line}"
        );
        assert!(line.contains("\"schema\":\"rynix.diag.v1\""), "{line}");
        assert!(line.contains("\"stage\":\"lex\""), "{line}");
    }
    // The `;` diagnostic must carry an auto-applicable removal fix.
    assert!(
        lines
            .iter()
            .any(|l| l.contains("unknown character `;`") && l.contains("\"confidence\":0.9")),
        "missing high-confidence fix in:\n{text}"
    );
}

#[test]
fn missing_file_is_exit_code_three() {
    let output = run(&["lex", "does/not/exist.ryx"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(stderr(&output).contains("cannot read"));
}

#[test]
fn help_and_version() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("Usage: rynixc"));

    let output = run(&["-V"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with("rynixc 0.1.0"));

    let output = run(&["nonsense"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown command"));
}

#[test]
fn parse_dumps_ast() {
    let path = corpus("hello.ryx");
    let output = run(&["parse", path.to_str().unwrap(), "--dump-ast"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let dump = stdout(&output);
    assert!(dump.contains("(fn main"), "{dump}");
    assert!(dump.contains("(call"), "{dump}");
    assert!(stderr(&output).is_empty());
}

#[test]
fn parse_reports_missing_end() {
    let dir = std::env::temp_dir();
    let path = dir.join("rynixc_parse_missing_end.ryx");
    std::fs::write(&path, "def a()\n  return 1\n").unwrap();
    let output = run(&["parse", path.to_str().unwrap(), "--error-format=json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("RYX1004"), "{}", stderr(&output));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn check_clean_file_succeeds() {
    let path = corpus("hello.ryx");
    let output = run(&["check", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).is_empty());
}

#[test]
fn check_broken_file_json_is_schema_valid() {
    let path = corpus("errors.ryx");
    let output = run(&["check", path.to_str().unwrap(), "--error-format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "expected diagnostics");
    for line in lines {
        assert!(line.contains("\"schema\":\"rynix.diag.v1\""), "{line}");
        assert!(
            line.contains("\"stage\":\"lex\"") || line.contains("\"stage\":\"parse\""),
            "{line}"
        );
    }
}

#[test]
fn check_human_shows_snippet() {
    let dir = std::env::temp_dir();
    let path = dir.join("rynixc_check_snippet.ryx");
    std::fs::write(&path, "x $ y\n").unwrap();
    let output = run(&["check", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.contains("error[RYX0001]"), "{text}");
    assert!(
        text.contains("| x $ y") || text.contains("1 | x $ y"),
        "{text}"
    );
    assert!(text.contains('^'), "{text}");
    let _ = std::fs::remove_file(path);
}
