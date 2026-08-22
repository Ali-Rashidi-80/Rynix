//! Honest probe: bool `and`/`or` must lower to bool-typed ops usable in `if`.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{interpret_module, lower_module, run_pipeline, InterpValue};
use rynix_sema::analyze;
use rynix_span::Interner;

fn run(src: &str) -> i64 {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    match interpret_module(&rir, &interner).expect("interp") {
        InterpValue::I64(n) => n,
        other => panic!("expected i64, got {other:?}"),
    }
}

#[test]
fn bool_and_false_short_branch() {
    let src = "def main() -> i64\n  if true and false\n    return 1\n  end\n  return 0\nend\n";
    assert_eq!(run(src), 0);
}

#[test]
fn bool_or_true_branch() {
    let src = "def main() -> i64\n  if false or true\n    return 1\n  end\n  return 0\nend\n";
    assert_eq!(run(src), 1);
}

#[test]
fn bool_and_true_else() {
    let src = "def main() -> i64\n  if true and true\n    return 7\n  else\n    return 8\n  end\nend\n";
    assert_eq!(run(src), 7);
}

#[test]
fn bool_and_or_in_match() {
    let src = r"
def main() -> i64
  let b = true and false
  match b
    true
      return 1
    false
      return 2
  end
  return 0
end
";
    assert_eq!(run(src), 2);
}
