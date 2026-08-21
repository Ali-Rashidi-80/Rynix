//! Comment-directive tests: `#^ error RYX2xxx` on the same source line.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use std::collections::HashSet;

#[derive(Debug)]
struct Expectation {
    line: u32,
    code: String,
}

fn parse_directives(src: &str) -> Vec<Expectation> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let line_no = (i + 1) as u32;
        // Look for `#^ error RYX####` inside a `#` comment.
        if let Some(idx) = line.find("#^") {
            let rest = &line[idx + 2..];
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix("error") {
                let code = rest.split_whitespace().next().unwrap_or("");
                if code.starts_with("RYX") {
                    out.push(Expectation {
                        line: line_no,
                        code: code.to_string(),
                    });
                }
            }
        }
    }
    out
}

fn check_file(src: &str) {
    let expects = parse_directives(src);
    assert!(!expects.is_empty(), "no #^ directives found");

    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);

    let mut sm = SourceMap::new();
    sm.add_owned("directive.ryx", src.to_string());

    let mut unmatched: HashSet<(u32, String)> =
        expects.iter().map(|e| (e.line, e.code.clone())).collect();

    for diag in &sink.diags {
        let (_, lc) = sm.line_col(diag.primary.span.lo());
        unmatched.remove(&(lc.line, diag.code.as_str().to_string()));
    }

    assert!(
        unmatched.is_empty(),
        "unmet directives: {unmatched:?}\nactual: {:?}",
        sink.diags
            .iter()
            .map(|d| {
                let (_, lc) = sm.line_col(d.primary.span.lo());
                (lc.line, d.code.as_str())
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn directives_unresolved_and_mismatch() {
    check_file(
        "\
def f() -> i64
  return missing #^ error RYX2001
end

def g() -> i64
  return true #^ error RYX2003
end
",
    );
}

#[test]
fn directives_immutable_and_arity() {
    check_file(
        "\
def add(a: i64, b: i64) -> i64
  return a + b
end

def h()
  let x = 1
  x = 2 #^ error RYX2005
  let _ = add(1) #^ error RYX2007
end
",
    );
}
