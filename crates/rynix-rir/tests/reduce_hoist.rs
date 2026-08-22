//! Reduce loop body: invariant `iconst` hoisted out of the latch block.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const REDUCE_MAIN: &str = include_str!("../../../benchmarks/suite5/reduce.ryx");

#[test]
fn reduce_hoists_loop_invariant_iconsts_to_entry() {
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
        text.contains("block0:") && text.contains("iconst 13"),
        "expected hoisted mod-13 constant in entry:\n{text}"
    );
    let body = text
        .split("block2:")
        .nth(1)
        .and_then(|s| s.split("block3:").next())
        .unwrap_or("");
    assert!(
        !body.contains("iconst 13") && !body.contains("iconst 5"),
        "loop latch should not re-emit invariant constants:\n{text}"
    );
}
