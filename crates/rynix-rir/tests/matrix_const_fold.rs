//! Literal-bound matrix kernel folds; Suite5 uses opaque `per` (see suite5_const_kernels).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const MATRIX_LITERAL: &str = r#"
def cell(i: i64, j: i64) -> i64
  let mut s = 0
  let mut k = 0
  loop
    if k >= 4
      break
    end
    let av = i + k
    let bv = k * j + 1
    s += av * bv
    k += 1
  end
  return s
end

def main() -> i64
  let c00 = cell(0, 0)
  let c11 = cell(1, 1)
  let c22 = cell(2, 2)
  let c33 = cell(3, 3)
  let per = 225000
  let trace = per * (c00 + c11 + c22 + c33)
  print_i64(trace)
  return 0
end
"#;

#[test]
fn matrix_folds_to_checksum_constant() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, MATRIX_LITERAL, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, MATRIX_LITERAL, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("48600000"),
        "expected matrix trace constant 48600000 after const-fold:\n{text}"
    );
}
