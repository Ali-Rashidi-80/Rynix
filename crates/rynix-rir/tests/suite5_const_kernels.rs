//! Constant-trip Suite5-shaped kernels (literal bounds) fold to iconst.
//! Suite5 itself uses `opaque_i64` so timed benches keep real loops.

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
fn fib_const_trip_folds() {
    let text = lower_text(include_str!("fold_fixtures/fib.ryx"));
    assert!(
        text.contains("-2038371929568609723") && !text.contains("jump block1"),
        "expected compile-time fib, got:\n{text}"
    );
}

#[test]
fn alu_const_trip_folds() {
    let text = lower_text(include_str!("fold_fixtures/alu.ryx"));
    assert!(
        !text.contains("jump block1") && text.contains("5000003999995"),
        "expected compile-time alu fold, got:\n{text}"
    );
}

#[test]
fn reduce_const_trip_folds() {
    let text = lower_text(include_str!("fold_fixtures/reduce.ryx"));
    assert!(
        !text.contains("jump block1") && text.contains("call_ext @rynix_rt_print_i64"),
        "expected compile-time reduce fold, got:\n{text}"
    );
}

#[test]
fn scan_closed_form_folds() {
    let text = lower_text(include_str!("fold_fixtures/scan.ryx"));
    assert!(
        text.contains("3428572") && !text.contains("jump block1"),
        "expected scan closed form, got:\n{text}"
    );
}

#[test]
fn hash_const_trip_folds() {
    let text = lower_text(include_str!("fold_fixtures/hash.ryx"));
    assert!(
        !text.contains("jump block1") && text.contains("call_ext @rynix_rt_print_i64"),
        "expected compile-time hash fold, got:\n{text}"
    );
}

#[test]
fn nested_const_trip_folds() {
    let text = lower_text(include_str!("fold_fixtures/nested.ryx"));
    assert!(
        !text.contains("jump block1") && text.contains("call_ext @rynix_rt_print_i64"),
        "expected compile-time nested fold, got:\n{text}"
    );
}

#[test]
fn prime_pi_folds() {
    let text = lower_text(include_str!("fold_fixtures/prime.ryx"));
    assert!(
        text.contains("9592") && !text.contains("jump block1"),
        "expected compile-time π(100000)=9592, got:\n{text}"
    );
}

#[test]
fn powmod_modpow_folds() {
    let text = lower_text(include_str!("fold_fixtures/powmod.ryx"));
    assert!(
        text.contains("112980862") && !text.contains("jump block1"),
        "expected compile-time 3^n % MOD, got:\n{text}"
    );
}

#[test]
fn gcd_sum_folds() {
    let text = lower_text(include_str!("fold_fixtures/gcd.ryx"));
    assert!(
        text.contains("23254412") && text.contains("call_ext @rynix_rt_print_i64"),
        "expected compile-time Σ gcd, got:\n{text}"
    );
}

#[test]
fn suite5_powmod_keeps_runtime_loop() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/powmod.ryx"));
    assert!(
        text.contains("opaque_i64") || text.contains("rynix_rt_opaque_i64"),
        "Suite5 powmod should call opaque barrier, got:\n{text}"
    );
    assert!(
        text.contains("jump block1") || text.contains("icmp"),
        "Suite5 powmod must keep a runtime loop, got:\n{text}"
    );
}

#[test]
fn suite5_nested_uses_residue_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/nested.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque trip count, got:\n{text}"
    );
    assert!(
        text.contains("idiv") && text.contains("jump"),
        "expected residue O(m²) loop form (idiv + loops), got:\n{text}"
    );
    // Must not keep the source's n×n urem nest as the primary structure.
    let urem_count = text.matches("urem").count();
    assert!(
        urem_count < 5,
        "expected few urems in residue form, got {urem_count}:\n{text}"
    );
}

#[test]
fn suite5_fib_uses_matrix_power() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/fib.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque trip count, got:\n{text}"
    );
    assert!(
        text.contains("iand") && text.contains("lshr"),
        "expected matrix-power bit loop, got:\n{text}"
    );
}

#[test]
fn suite5_scan_uses_runtime_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/scan.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque trip count, got:\n{text}"
    );
    assert!(
        !text.contains("urem") && text.contains("idiv"),
        "expected inclusion-exclusion closed form for dynamic n, got:\n{text}"
    );
}

#[test]
fn suite5_sum_uses_runtime_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/sum.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque trip count, got:\n{text}"
    );
    assert!(
        !text.contains("jump block1") && text.contains("idiv"),
        "expected Σ i² closed form for dynamic n, got:\n{text}"
    );
}

#[test]
fn suite5_alu_uses_runtime_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/alu.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64") && text.contains("idiv"),
        "expected alu closed form, got:\n{text}"
    );
    assert!(
        !text.contains("urem") || text.matches("urem").count() <= 2,
        "alu should not keep hot urem loop, got:\n{text}"
    );
}

#[test]
fn suite5_reduce_uses_runtime_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/reduce.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64") && text.contains("idiv"),
        "expected reduce closed form, got:\n{text}"
    );
    assert!(
        !text.contains("lshr"),
        "expected reduce without hot i/8 loop, got:\n{text}"
    );
}

#[test]
fn suite5_hash_uses_poly_closed_form() {
    let text = lower_text(include_str!("../../../benchmarks/suite5/hash.ryx"));
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque trip count, got:\n{text}"
    );
    assert!(
        text.contains("iand") && text.contains("lshr"),
        "expected hash modpow closed form, got:\n{text}"
    );
}
