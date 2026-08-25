//! Suite5 scan/hash lowering patterns (opaque → closed forms).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const SCAN_MAIN: &str = include_str!("../../../benchmarks/suite5/scan.ryx");
const HASH_MAIN: &str = include_str!("../../../benchmarks/suite5/hash.ryx");

#[test]
fn scan_opaque_uses_div_closed_form() {
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
        text.contains("opaque_i64")
            && (text.contains("idiv") || text.contains("udiv") || text.contains("urem")),
        "expected opaque scan → divisibility closed form:\n{text}"
    );
}

#[test]
fn hash_opaque_closed_form_keeps_imul_and_rem() {
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
        text.contains("imul") && text.contains("lshr"),
        "expected hash closed-form modpow (imul + lshr):\n{text}"
    );
    assert!(
        text.contains("urem") || text.contains("irem"),
        "expected rem in hash closed form:\n{text}"
    );
}
