//! Differential oracle: RIR interpreter vs expected values for small programs.

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
fn diff_arith() {
    assert_eq!(
        eval("def main() -> i64\n  return 2 + 3 * 4\nend\n"),
        InterpValue::I64(14)
    );
}

#[test]
fn diff_array_index() {
    assert_eq!(
        eval("def main() -> i64\n  let a = [10, 20, 30]\n  return a[2]\nend\n"),
        InterpValue::I64(30)
    );
}

#[test]
fn diff_for_sum() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let mut s = 0
  for x in [1, 2, 3]
    s += x
  end
  return s
end
"
        ),
        InterpValue::I64(6)
    );
}

#[test]
fn diff_break() {
    assert_eq!(
        eval(
            r"
def main() -> i64
  let mut i = 0
  loop
    if i == 3
      break
    end
    i += 1
  end
  return i
end
"
        ),
        InterpValue::I64(3)
    );
}
