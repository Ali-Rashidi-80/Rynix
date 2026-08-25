//! Unit and directive tests for semantic analysis.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_sema::{analyze, dump_types};
use rynix_span::Interner;

fn run(src: &str) -> (String, VecSink) {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    let analysis = analyze(module, &mut interner, &mut sink);
    let dump = dump_types(module, &analysis, &interner);
    (dump, sink)
}

fn codes(sink: &VecSink) -> Vec<&str> {
    sink.diags.iter().map(|d| d.code.as_str()).collect()
}

#[test]
fn simple_function_types() {
    let (dump, sink) = run("\
def add(a: i64, b: i64) -> i64
  return a + b
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    assert!(dump.contains("fn add: fn(i64, i64) -> i64"), "{dump}");
}

#[test]
fn let_inference() {
    let (dump, sink) = run("\
def f() -> i64
  let x = 1
  let y = x
  return y
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    assert!(dump.contains("let x: i64"), "{dump}");
    assert!(dump.contains("let y: i64"), "{dump}");
}

#[test]
fn unresolved_name() {
    let (_, sink) = run("\
def f() -> i64
  return missing
end
");
    assert!(codes(&sink).contains(&"RYX2001"), "{:?}", sink.diags);
}

#[test]
fn type_mismatch_return() {
    let (_, sink) = run("\
def f() -> i64
  return true
end
");
    assert!(codes(&sink).contains(&"RYX2003"), "{:?}", sink.diags);
}

#[test]
fn immutable_assign() {
    let (_, sink) = run("\
def f()
  let x = 1
  x = 2
end
");
    assert!(codes(&sink).contains(&"RYX2005"), "{:?}", sink.diags);
}

#[test]
fn mutable_assign_ok() {
    let (_, sink) = run("\
def f()
  let mut x = 1
  x = 2
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
}

#[test]
fn wrong_arity() {
    let (_, sink) = run("\
def add(a: i64, b: i64) -> i64
  return a + b
end
def g() -> i64
  return add(1)
end
");
    assert!(codes(&sink).contains(&"RYX2007"), "{:?}", sink.diags);
}

#[test]
fn break_outside_loop() {
    let (_, sink) = run("\
def f()
  break
end
");
    assert!(codes(&sink).contains(&"RYX2008"), "{:?}", sink.diags);
}

#[test]
fn struct_field_access() {
    let (dump, sink) = run("\
struct Point
  x: i64
  y: i64
end
def get_x(p: Point) -> i64
  return p.x
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    assert!(dump.contains("fn get_x: fn(Point) -> i64"), "{dump}");
}

#[test]
fn unknown_field() {
    let (_, sink) = run("\
struct Point
  x: i64
end
def get_z(p: Point) -> i64
  return p.z
end
");
    assert!(codes(&sink).contains(&"RYX2006"), "{:?}", sink.diags);
}

#[test]
fn duplicate_fn() {
    let (_, sink) = run("\
def f()
end
def f()
end
");
    assert!(codes(&sink).contains(&"RYX2002"), "{:?}", sink.diags);
}

#[test]
fn use_after_move_vec_let() {
    let (_, sink) = run("\
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  let w = v
  return v.len()
end
");
    assert!(codes(&sink).contains(&"RYX2011"), "{:?}", sink.diags);
}

#[test]
fn use_after_move_call_arg() {
    let (_, sink) = run("\
def take(v: Vec[i64]) -> i64
  return v.len()
end
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  let _n = take(v)
  return v.len()
end
");
    assert!(codes(&sink).contains(&"RYX2011"), "{:?}", sink.diags);
}

#[test]
fn move_reinit_allows_use() {
    let (_, sink) = run("\
def main() -> i64
  let mut v: Vec[i64] = vec_new(0)
  let w = v
  v = vec_new(0)
  return v.len() + w.len()
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
}

#[test]
fn i64_copy_not_move() {
    let (_, sink) = run("\
def main() -> i64
  let x = 1
  let y = x
  return x + y
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
}

#[test]
fn field_assign_rejected() {
    // Index assign remains RYX2020; field store ships in Wave 3.
    let (_, sink) = run("\
def main() -> i64
  let a: [i64] = [1, 2]
  a[0] = 9
  return 0
end
");
    assert!(codes(&sink).contains(&"RYX2020"), "{:?}", sink.diags);
}

#[test]
fn field_assign_immutable_rejected() {
    let (_, sink) = run("\
struct Point
  x: i64
  y: i64
end
def set_x(p: Point) -> i64
  p.x = 1
  return 0
end
");
    assert!(codes(&sink).contains(&"RYX2005"), "{:?}", sink.diags);
}

#[test]
fn struct_literal_and_field_store() {
    let (dump, sink) = run("\
struct Point
  x: i64
  y: i64
end
def main() -> i64
  let mut p = Point { x: 1, y: 2 }
  p.x = 10
  return p.x + p.y
end
");
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    assert!(dump.contains("fn main: fn() -> i64"), "{dump}");
}

#[test]
fn stub_reserved_rejected() {
    let (_, sink) = run("\
def main() -> i64
  signal(1)
  return 0
end
");
    assert!(codes(&sink).contains(&"RYX2013"), "{:?}", sink.diags);
}
