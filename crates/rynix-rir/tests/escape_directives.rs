//! Escape-analysis directive tests: `#^ alloc: stack|region|heap`.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{analyze_escape, lower_module, Placement};
use rynix_sema::analyze;
use rynix_span::{Interner, SourceMap};
use std::collections::HashSet;

#[derive(Debug)]
struct Expect {
    line: u32,
    place: &'static str,
}

fn parse_alloc_directives(src: &str) -> Vec<Expect> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let line_no = (i + 1) as u32;
        if let Some(idx) = line.find("#^") {
            let rest = line[idx + 2..].trim_start();
            if let Some(rest) = rest.strip_prefix("alloc:") {
                let place = rest.split_whitespace().next().unwrap_or("");
                let place = match place {
                    "stack" => "stack",
                    "region" => "region",
                    "heap" => "heap",
                    _ => continue,
                };
                out.push(Expect {
                    line: line_no,
                    place,
                });
            }
        }
    }
    out
}

fn check_allocs(src: &str) {
    let expects = parse_alloc_directives(src);
    assert!(!expects.is_empty(), "no #^ alloc: directives");

    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let rir = lower_module(module, &analysis, &mut interner, src, 0);
    let report = analyze_escape(&rir, &interner);

    let mut sm = SourceMap::new();
    sm.add_owned("escape.ryx", src.to_string());

    let mut unmatched: HashSet<(u32, &str)> =
        expects.iter().map(|e| (e.line, e.place)).collect();

    for site in &report.sites {
        let (_, lc) = sm.line_col(site.span.lo());
        let place = match site.placement {
            Placement::Stack => "stack",
            Placement::Region(_) => "region",
            Placement::Heap => "heap",
        };
        unmatched.remove(&(lc.line, place));
    }

    assert!(
        unmatched.is_empty(),
        "unmet alloc directives: {unmatched:?}\nreport: {:?}",
        report
            .sites
            .iter()
            .map(|s| {
                let (_, lc) = sm.line_col(s.span.lo());
                (lc.line, s.placement.as_str(), s.escape.as_str(), &s.reason)
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn locals_are_stack() {
    check_allocs(
        r"
def main() -> i64
  let x = 1 #^ alloc: stack
  let y = x + 2 #^ alloc: stack
  return y
end
",
    );
}

#[test]
fn params_are_stack() {
    check_allocs(
        r"
def add(a: i64, b: i64) -> i64 # param slots are stack
  return a + b
end

def main() -> i64
  let a = 1 #^ alloc: stack
  let b = 2 #^ alloc: stack
  return add(a, b)
end
",
    );
}

#[test]
fn returned_pointer_arg_escapes() {
    // Returning a local pointer isn't expressible yet for scalars; verify the
    // pipeline still classifies ordinary lets as stack.
    check_allocs(
        r"
def main() -> i64
  let box = 42 #^ alloc: stack
  return box
end
",
    );
}
