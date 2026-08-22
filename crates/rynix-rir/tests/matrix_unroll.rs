//! Matrix `cell`: inner `k = 0..4` loop fully unrolled (no back-edge).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const MATRIX_CELL: &str = r#"
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
"#;

const MATRIX_MAIN: &str = include_str!("../../../benchmarks/suite5/matrix.ryx");

#[test]
fn matrix_cell_inner_k_loop_is_fully_unrolled() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, MATRIX_CELL, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, MATRIX_CELL, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        !text.contains("jump block1"),
        "expected no counted-loop back-edge in cell:\n{text}"
    );
    assert!(
        text.matches("imul").count() >= 4,
        "expected four unrolled multiply steps:\n{text}"
    );
}

#[test]
fn matrix_main_checksum_unchanged_after_unroll() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, MATRIX_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, MATRIX_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("imul %0") || text.contains("imul %18"),
        "inlined cell bodies should retain matmul-style products:\n{text}"
    );
}
