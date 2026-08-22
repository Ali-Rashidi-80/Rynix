//! Strength-reduce `x * 31` → `(x << 5) - x` for non-negative `x`.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const HASH_BODY: &str = r#"
def main() -> i64
  let n = 3000000
  let mut h = 0
  let mut i = 0
  loop
    if i >= n
      break
    end
    h = (h * 31 + i) % 1000000007
    i += 1
  end
  print_i64(h)
  return 0
end
"#;

#[test]
fn hash_mul31_strength_reduced_to_lshl() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, HASH_BODY, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, HASH_BODY, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("lshl") && !text.contains("imul %5, %8"),
        "expected lshl instead of imul by 31, got:\n{text}"
    );
}
