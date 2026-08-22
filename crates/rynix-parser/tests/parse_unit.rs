//! Unit tests for the recursive-descent + Pratt parser.

use rynix_ast::{AssignOp, AstArena, Item, Stmt, dump_module};
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_span::Interner;

fn parse_ok(src: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert!(
        sink.error_count() == 0,
        "unexpected errors for {src:?}: {:?}",
        sink.diags
    );
    dump_module(module, &interner)
}

#[test]
fn hello_main() {
    let dump = parse_ok(
        "\
def main()
  print(\"Hello\")
end
",
    );
    assert!(dump.contains("(fn main"), "{dump}");
    assert!(dump.contains("(call"), "{dump}");
    assert!(dump.contains("(path print)"), "{dump}");
    assert!(dump.contains("(str)"), "{dump}");
}

#[test]
fn precedence_mul_over_add() {
    let dump = parse_ok(
        "\
def f()
  return 1 + 2 * 3
end
",
    );
    let plus = dump.find("(binary +").expect("plus");
    let mul = dump.find("(binary *").expect("mul");
    assert!(mul > plus, "mul should be nested under plus:\n{dump}");
}

#[test]
fn comparisons_non_associative() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let _ = parse(
        &arena,
        &mut interner,
        "def f()\n  return a < b < c\nend\n",
        0,
        &mut sink,
    );
    assert!(
        sink.diags.iter().any(|d| d.code.as_str() == "RYX1007"),
        "{:?}",
        sink.diags
    );
}

#[test]
fn assignment_is_a_statement() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(
        &arena,
        &mut interner,
        "def f()\n  total += 1\nend\n",
        0,
        &mut sink,
    );
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let Item::Fn(f) = &module.items[0] else {
        panic!("expected fn");
    };
    let Stmt::Assign(a) = &f.body[0] else {
        panic!("expected assign, got {:?}", f.body[0]);
    };
    assert_eq!(a.op, AssignOp::PlusEq);
}

#[test]
fn unary_not_and_neg() {
    let dump = parse_ok("def f()\n  return not -x\nend\n");
    assert!(dump.contains("(unary not"), "{dump}");
    assert!(dump.contains("(unary -"), "{dump}");
}

#[test]
fn cast_and_spawn() {
    let dump = parse_ok(
        "\
def f()
  let x = total as f64
  spawn worker(x)
end
",
    );
    assert!(dump.contains("(as f64"), "{dump}");
    assert!(dump.contains("(spawn"), "{dump}");
}

#[test]
fn struct_enum_import_type() {
    let dump = parse_ok(
        "\
import std::io
type Id = i64
pub struct Point
  x: i64
  y: i64
end
enum Color
  Red
  Rgb(i64)
end
",
    );
    assert!(dump.contains("(import std::io)"), "{dump}");
    assert!(dump.contains("(type Id = i64)"), "{dump}");
    assert!(dump.contains("(pub struct Point"), "{dump}");
    assert!(dump.contains("(enum Color"), "{dump}");
}

#[test]
fn missing_end_recovers() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(
        &arena,
        &mut interner,
        "def a()\n  return 1\ndef b()\n  return 2\nend\n",
        0,
        &mut sink,
    );
    assert!(
        sink.diags.iter().any(|d| d.code.as_str() == "RYX1004"),
        "{:?}",
        sink.diags
    );
    assert!(module.items.len() >= 2, "recovered second fn");
}

#[test]
fn parse_match_stmt() {
    let dump = parse_ok(
        "\
def f(x: i64) -> i64
  match x
    1
      return 10
    _
      return 0
  end
end
",
    );
    assert!(dump.contains("(match"), "{dump}");
    assert!(dump.contains("(arm"), "{dump}");
}

#[test]
fn slice_type_and_array_expr() {
    let dump = parse_ok(
        "\
def f(xs: [i64])
  let ys = [1, 2, 3]
  return ys
end
",
    );
    assert!(dump.contains("xs: [i64]"), "{dump}");
    assert!(dump.contains("(array"), "{dump}");
}
