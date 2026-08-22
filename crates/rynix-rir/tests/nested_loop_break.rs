//! Nested loops: `break` passes outer carried syms to post-loop print (no stale inner phi).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const NESTED_MAIN: &str = include_str!("../../../benchmarks/suite5/nested.ryx");

#[test]
fn nested_break_passes_outer_carried_to_print() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, NESTED_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, NESTED_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("icmp lt %5, %0"),
        "outer loop should use guarded counted exit at header:\n{text}"
    );
    assert!(
        text.contains("block3(%3:i64, %4:i64)") && text.contains("call_ext @rynix_rt_print_i64(%4)"),
        "outer exit should wire carried acc into print block:\n{text}"
    );
    assert!(
        !text.contains("call_ext @rynix_rt_print_i64(%7)"),
        "print must not use stale inner-exit value:\n{text}"
    );
    assert!(
        text.contains("imul %5, %") && !text.contains("iadd %14, %5"),
        "i*j+i should fold to i*(j+1), not imul then add i:\n{text}"
    );
}
