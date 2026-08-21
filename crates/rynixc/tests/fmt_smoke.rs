//! Formatter CLI / library smoke tests.

use rynix_ast::{format_module, AstArena};
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_span::Interner;

#[test]
fn formats_hello_canonically() {
    let src = "def main()\n  print(\"Hello, Rynix\")\nend\n";
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0);
    let out = format_module(module, &interner, src, 0);
    assert_eq!(out, src);
}

#[test]
fn formats_messy_spacing() {
    let src = "def  add(a:i64,b:i64)->i64\nreturn a+b\nend\n";
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let out = format_module(module, &interner, src, 0);
    assert!(out.contains("def add(a: i64, b: i64) -> i64\n"), "{out}");
    assert!(out.contains("  return a + b\n"), "{out}");
    assert!(out.ends_with("end\n"), "{out}");
}
