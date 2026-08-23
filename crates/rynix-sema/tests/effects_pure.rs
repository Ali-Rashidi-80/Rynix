//! `#^ effect: pure` purity checking (A3).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_sema::analyze_with_source;
use rynix_span::Interner;

fn codes(src: &str) -> Vec<String> {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    let _ = analyze_with_source(module, &mut interner, &mut sink, Some(src), 0);
    sink.diags
        .iter()
        .map(|d| d.code.as_str().to_string())
        .collect()
}

#[test]
fn pure_ok_arithmetic() {
    let src = "\
def add(a: i64, b: i64) -> i64  #^ effect: pure
  return a + b
end
";
    let c = codes(src);
    assert!(!c.iter().any(|x| x == "RYX2012"), "{c:?}");
}

#[test]
fn pure_rejects_print() {
    let src = "\
def bad()  #^ effect: pure
  print_i64(1)
end
";
    let c = codes(src);
    assert!(c.iter().any(|x| x == "RYX2012"), "{c:?}");
}

#[test]
fn pure_rejects_transitive_http() {
    let src = "\
def fetch() -> i64
  return http_get_json_i64(\"127.0.0.1\", 80, \"/\", \"n\")
end

def wrapper() -> i64  #^ effect: pure
  return fetch()
end
";
    let c = codes(src);
    assert!(c.iter().any(|x| x == "RYX2012"), "{c:?}");
}

#[test]
fn unmarked_impure_ok() {
    let src = "\
def talk()
  print_i64(1)
end
";
    let c = codes(src);
    assert!(!c.iter().any(|x| x == "RYX2012"), "{c:?}");
}
