//! Suite5 scan/hash lowering patterns.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const SCAN_MAIN: &str = include_str!("../../../benchmarks/suite5/scan.ryx");
const HASH_MAIN: &str = include_str!("../../../benchmarks/suite5/hash.ryx");

#[test]
fn scan_uses_lazy_or_conditional_add() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, SCAN_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, SCAN_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("br") && text.contains("urem"),
        "expected short-circuit or + urem divisibility checks:\n{text}"
    );
}

#[test]
fn hash_uses_lshl_and_urem_mod() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, HASH_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, HASH_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("lshl") && text.contains("urem"),
        "expected ×31 strength reduce + unsigned mod:\n{text}"
    );
    assert!(
        !text.contains("srem"),
        "signed rem should not appear in hash hot loop:\n{text}"
    );
}
