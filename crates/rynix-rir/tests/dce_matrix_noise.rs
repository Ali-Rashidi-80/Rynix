//! DCE must strip dead pure ops from block bodies (not only rewrite to iconst 0).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_rir::{lower_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

#[test]
fn matrix_style_folds_without_dead_noise() {
    let src = r#"
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
  let per = opaque_i64(225000)
  let trace = per * (c00 + c11 + c22 + c33)
  print_i64(trace)
  return 0
end
"#;
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, src, 0, &mut sink);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "sema errors");
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    let errs = run_pipeline(&mut rir);
    assert!(errs.is_empty(), "{errs:?}");
    let main = rir.funcs.iter().find(|f| interner.resolve(f.name) == "main").unwrap();
    let mut iconst_zero_in_blocks = 0usize;
    for block in &main.blocks {
        for &iid in &block.insts {
            if matches!(main.inst(iid), rynix_rir::Inst::IConst(0)) {
                iconst_zero_in_blocks += 1;
            }
        }
    }
    // A handful of legitimate zeros (e.g. ret 0) is fine; hundreds is the old bug.
    assert!(
        iconst_zero_in_blocks < 8,
        "too many dead iconst 0 still in main blocks: {iconst_zero_in_blocks}"
    );
}
