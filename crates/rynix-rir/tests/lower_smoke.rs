//! Phase 5 RIR tests: lower + print + interpret + passes.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{
    interpret_module, lower_module, print_module, run_pipeline, verify_module, InterpValue,
};
use rynix_sema::analyze;
use rynix_span::Interner;

fn lower(src: &str) -> (rynix_rir::Module, Interner) {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "parse errors: {:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "sema errors: {:?}", sink.diags);
    let rir = lower_module(module, &analysis, &mut interner, src, 0);
    let errs = verify_module(&rir);
    assert!(errs.is_empty(), "verify: {errs:?}");
    (rir, interner)
}

#[test]
fn lower_add_and_return() {
    let src = r"
def main() -> i64
  return 1 + 2
end
";
    let (rir, interner) = lower(src);
    let text = print_module(&rir, &interner);
    assert!(text.contains("func @main"), "{text}");
    assert!(text.contains("iadd"), "{text}");
    assert!(text.contains("ret"), "{text}");

    let v = interpret_module(&rir, &interner).expect("interp");
    assert_eq!(v, InterpValue::I64(3));
}

#[test]
fn lower_if_else() {
    let src = r"
def main() -> i64
  if true
    return 10
  else
    return 20
  end
end
";
    let (rir, interner) = lower(src);
    let text = print_module(&rir, &interner);
    assert!(text.contains("br "), "{text}");
    let v = interpret_module(&rir, &interner).expect("interp");
    assert_eq!(v, InterpValue::I64(10));
}

#[test]
fn const_fold_pipeline() {
    let src = r"
def main() -> i64
  return 2 * 3 + 4
end
";
    let (mut rir, interner) = lower(src);
    let errs = run_pipeline(&mut rir);
    assert!(errs.is_empty(), "{errs:?}");
    let text = print_module(&rir, &interner);
    assert!(text.contains("iconst"), "{text}");
    let v = interpret_module(&rir, &interner).expect("interp");
    assert_eq!(v, InterpValue::I64(10));
}

#[test]
fn call_user_function() {
    let src = r"
def add(a: i64, b: i64) -> i64
  return a + b
end

def main() -> i64
  return add(7, 8)
end
";
    let (rir, interner) = lower(src);
    let v = interpret_module(&rir, &interner).expect("interp");
    assert_eq!(v, InterpValue::I64(15));
}
