//! Reduce: nonneg `i`/`acc` enable urem and lshr in the hot loop.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const REDUCE_MAIN: &str = include_str!("../../../benchmarks/suite5/reduce.ryx");

#[test]
fn reduce_loop_uses_urem_and_lshr_for_nonneg_i() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, REDUCE_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, REDUCE_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("urem") && text.contains("lshr") && (text.contains("lshl") || text.contains("imul")),
        "expected nonneg strength reductions in reduce:\n{text}"
    );
    assert!(
        !text.contains("srem") && !text.contains("ashr"),
        "signed div/rem should not appear for nonneg i:\n{text}"
    );
    assert!(
        !text.contains("__incr_mod_"),
        "incremental mod blocks LLVM vectorization; use direct urem on counter:\n{text}"
    );
}
