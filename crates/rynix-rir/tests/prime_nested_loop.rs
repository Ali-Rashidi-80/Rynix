//! Prime: Suite5 uses opaque limit (runtime trial); literal limit still folds to π(n).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

fn lower_pipeline(src: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    print_module(&rir, &interner)
}

#[test]
fn prime_literal_folds_to_pi() {
    let text = lower_pipeline(include_str!("fold_fixtures/prime.ryx"));
    assert!(
        text.contains("9592") && !text.contains("urem"),
        "expected host π(100000)=9592 fold, got:\n{text}"
    );
}

#[test]
fn prime_suite5_keeps_trial_division() {
    let text = lower_pipeline(include_str!("../../../benchmarks/suite5/prime.ryx"));
    assert!(
        text.contains("urem"),
        "Suite5 opaque limit should keep trial urem:\n{text}"
    );
    assert!(
        text.contains("imul") && text.contains("icmp le"),
        "expected inner j*j square guard:\n{text}"
    );
}
