//! Match statements and method-call lowering.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{interpret_module, lower_module, run_pipeline, InterpValue};
use rynix_sema::analyze;
use rynix_span::Interner;

fn eval(src: &str) -> InterpValue {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    let errs = run_pipeline(&mut rir);
    assert!(errs.is_empty(), "{errs:?}");
    interpret_module(&rir, &interner).expect("interp")
}

#[test]
fn match_int_arms() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let x = 2
  match x
    1
      return 10
    2
      return 20
    _
      return 0
  end
  return -1
end
"
        ),
        InterpValue::I64(20)
    );
}

#[test]
fn match_else() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let x = 9
  match x
    1
      return 1
    else
      return 99
  end
  return 0
end
"
        ),
        InterpValue::I64(99)
    );
}

#[test]
fn match_bool() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let b = true
  match b
    false
      return 0
    true
      return 7
  end
  return -1
end
"
        ),
        InterpValue::I64(7)
    );
}

#[test]
fn slice_len_method() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let a = [1, 2, 3, 4]
  return a.len()
end
"
        ),
        InterpValue::I64(4)
    );
}

fn analyze_only(src: &str) -> VecSink {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "parse: {:?}", sink.diags);
    let _ = analyze(module, &mut interner, &mut sink);
    sink
}

#[test]
fn vec_i64_annotation_accepted() {
    let sink = analyze_only(
        r"
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  return 0
end
",
    );
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
}

#[test]
fn vec_str_annotation_ok() {
    let sink = analyze_only(
        r"
def main() -> i64
  let v: Vec[str] = vec_str_new(0)
  return 0
end
",
    );
    assert_eq!(
        sink.error_count(),
        0,
        "Vec[str] + vec_str_new should typecheck: {:?}",
        sink.diags
    );
}

#[test]
fn vec_insert_method_rejected() {
    let sink = analyze_only(
        r"
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  v.insert(1, 2)
  return 0
end
",
    );
    assert!(
        sink.error_count() > 0,
        "Vec.insert must be rejected, got {:?}",
        sink.diags
    );
}

#[test]
fn map_push_method_rejected() {
    let sink = analyze_only(
        r"
def main() -> i64
  let m: Map[i64, i64] = map_new(0)
  m.push(1)
  return 0
end
",
    );
    assert!(
        sink.error_count() > 0,
        "Map.push must be rejected, got {:?}",
        sink.diags
    );
}

#[test]
fn bool_and_or_if() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  if true and false
    return 1
  end
  if false or true
    return 42
  end
  return 0
end
"
        ),
        InterpValue::I64(42)
    );
}
