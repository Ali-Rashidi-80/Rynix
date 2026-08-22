//! Prime outer loop with nested inner trial division uses phi loop (not nested guarded).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const PRIME_MAIN: &str = include_str!("../../../benchmarks/suite5/prime.ryx");

#[test]
fn prime_outer_loop_is_phi_not_guarded_with_nested_inner() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, PRIME_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, PRIME_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("urem"),
        "inner trial division should use urem:\n{text}"
    );
    assert!(
        text.contains("block1(%") && text.contains("imul"),
        "expected outer phi loop + inner j*j guard:\n{text}"
    );
}
