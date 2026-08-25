//! `×31` stays `imul` (feeds urem magic); `×3` may still strength-reduce.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const HASH_BODY: &str = include_str!("../../../benchmarks/suite5/hash.ryx");

#[test]
fn hash_mul31_keeps_imul_before_urem() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, HASH_BODY, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, HASH_BODY, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    // Opaque Suite5 hash lowers to a closed-form modpow (imul + rem). Keep ×31 as
    // `imul` (not shift-strength-reduced); rem may be `urem` or `irem`.
    assert!(
        text.contains("imul") && (text.contains("urem") || text.contains("irem")),
        "expected imul + rem for hash ×31:\n{text}"
    );
}
