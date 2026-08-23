//! Euclidean gcd → binary GCD (`cttz`); literal Suite5-shaped sum still host-folds.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module};
use rynix_sema::analyze;
use rynix_span::Interner;

fn lower_text(src: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let rir = lower_module(module, &analysis, &mut interner, src, 0);
    print_module(&rir, &interner)
}

#[test]
fn gcd_uses_binary_cttz() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/gcd.ryx"));
    assert!(
        text.contains("cttz") && !text.contains("urem") && !text.contains("irem"),
        "expected Stein binary gcd (cttz), got:\n{text}"
    );
}

#[test]
fn gcd_literal_sum_folds() {
    let text = lower_text(include_str!("fold_fixtures/gcd.ryx"));
    assert!(
        text.contains("23254412") && text.contains("call_ext @rynix_rt_print_i64"),
        "expected host Σ gcd fold to Suite5 checksum, got:\n{text}"
    );
}
