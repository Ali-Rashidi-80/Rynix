//! Sum-of-squares counted loops lower to a closed form.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module};
use rynix_sema::analyze;
use rynix_span::Interner;

#[test]
fn sum_of_squares_closed_form() {
    let src = include_str!("../../../benchmarks/suite5/sum.ryx");
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let rir = lower_module(module, &analysis, &mut interner, src, 0);
    let text = print_module(&rir, &interner);
    // n=1500000 → (n-1)*n*(2n-1)/6 = 1124998875000250000
    assert!(
        text.contains("1124998875000250000") && !text.contains("jump block1"),
        "expected closed-form sum of squares, got:\n{text}"
    );
}
