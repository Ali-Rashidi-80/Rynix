//! powmod: literal bounds fold to modpow; Suite5 opaque keeps rem peephole loop.

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
fn powmod_literal_folds_to_modpow() {
    let text = lower_text(include_str!("fold_fixtures/powmod.ryx"));
    assert!(
        text.contains("112980862") && !text.contains("jump block1"),
        "expected compile-time 3^n % MOD, got:\n{text}"
    );
}

#[test]
fn powmod_suite5_rem_uses_conditional_sub() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/powmod.ryx"));
    assert!(
        !text.contains("urem") && text.contains("isub") && text.contains("icmp"),
        "expected small-factor rem peephole in Suite5 loop, got:\n{text}"
    );
}
