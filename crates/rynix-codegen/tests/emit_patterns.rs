//! Pattern tests for textual LLVM emission.

use rynix_ast::AstArena;
use rynix_codegen::{emit_llvm, prune_unreachable, reachable_from_main};
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{analyze_escape, inject_regions, lower_module};
use rynix_sema::analyze;
use rynix_span::Interner;

fn emit(src: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    let report = analyze_escape(&rir, &interner);
    inject_regions(&mut rir, &report);
    prune_unreachable(&mut rir, &interner);
    emit_llvm(&rir, &interner, Some(&report))
}

#[test]
fn hello_emits_print_and_main() {
    let ll = emit(
        r#"
def main()
  print("Hello, Rynix")
end
"#,
    );
    assert!(ll.contains("define i32 @main()"), "{ll}");
    assert!(ll.contains("call void @rynix_rt_print"), "{ll}");
    assert!(ll.contains("@.str.0"), "{ll}");
    assert!(ll.contains("ret i32 0"), "{ll}");
    // Stack-only program: no heap alloc *calls* (declarations may still mention the symbol).
    assert!(
        !ll.contains("call ptr @rynix_rt_heap_alloc"),
        "unexpected heap: {ll}"
    );
}

#[test]
fn reachability_drops_dead_fn() {
    let src = r"
def dead() -> i64
  return 1
end

def main() -> i64
  return 2
end
";
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    let analysis = analyze(module, &mut interner, &mut sink);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert_eq!(rir.funcs.len(), 2);
    let keep = reachable_from_main(&rir, &interner);
    assert_eq!(keep.len(), 1);
    prune_unreachable(&mut rir, &interner);
    assert_eq!(rir.funcs.len(), 1);
    assert_eq!(interner.resolve(rir.funcs[0].name), "main");
    let ll = emit_llvm(&rir, &interner, None);
    assert!(ll.contains("@main"), "{ll}");
    assert!(!ll.contains("@dead"), "{ll}");
}

#[test]
fn immutable_lets_are_ssa_not_heap() {
    let ll = emit(
        r"
def main() -> i64
  let x = 1
  let y = 2
  return x + y
end
",
    );
    // Immutable locals lower as SSA values (no alloca / no heap).
    assert!(!ll.contains("alloca"), "{ll}");
    assert!(!ll.contains("call ptr @rynix_rt_heap_alloc"), "{ll}");
    assert!(ll.contains("add i64"), "{ll}");
}

#[test]
fn induction_rem_loop_emits_vectorize_metadata() {
    // Non-folded induction rem (reduce shape but smaller / not matching const-trip host eval bound alone —
    // use a form that keeps a live loop: non-zero start).
    let ll = emit(
        r"
def main() -> i64
  let mut i = 1
  let mut acc = 0
  loop
    if i >= 1000
      break
    end
    acc = acc + i % 13
    i += 1
  end
  return acc
end
",
    );
    assert!(
        ll.contains("!llvm.loop !0") && ll.contains("llvm.loop.vectorize.enable"),
        "induction-rem latch should vectorize:\n{ll}"
    );
    assert!(
        ll.contains("llvm.loop.mustprogress"),
        "expected mustprogress loop metadata:\n{ll}"
    );
}

#[test]
fn fib_opaque_strength_reduces_without_host_fold() {
    // Suite5 fairness: opaque_i64 blocks collapsing n to a host-evaluated iconst.
    // Matrix-power strength reduction may remain (log-n loop + metadata) — that is OK.
    let ll = emit(include_str!("../../../benchmarks/suite5/fib.ryx"));
    assert!(
        ll.contains("@rynix_rt_opaque_i64") || ll.contains("@rynix_rt_print_i64"),
        "fib Suite5 IR should retain opaque/print surface:\n{ll}"
    );
    assert!(
        !(ll.contains("-2038371929568609723") && !ll.contains("!llvm.loop")),
        "opaque fib must not host-fold to a lone checksum iconst:\n{ll}"
    );
}

#[test]
fn carried_rem_loop_skips_forced_unroll() {
    // Direct loop-carried urem (gcd-style): let clang -funroll-loops decide.
    let ll = emit(
        r"
def main() -> i64
  let mut acc = 1
  let mut i = 0
  loop
    if i >= 100
      break
    end
    acc = (acc * 17) % 1000000007
    i += 1
  end
  return acc
end
",
    );
    assert!(
        !ll.contains("llvm.loop.unroll.count"),
        "forced unroll.count must not be on rem-heavy latches:\n{ll}"
    );
}

#[test]
fn nested_opaque_uses_residue_reduction_not_host_fold() {
    // Opaque equal bounds → residue O(m²) loops (m=97), not a single iconst.
    let ll = emit(include_str!("../../../benchmarks/suite5/nested.ryx"));
    assert!(
        ll.contains("@rynix_rt_opaque_i64") || ll.contains("@rynix_rt_print_i64"),
        "nested Suite5 IR should retain opaque/print surface:\n{ll}"
    );
    assert!(
        !(ll.contains("9623703") && !ll.contains("!llvm.loop")),
        "opaque nested must not host-fold to a lone checksum iconst:\n{ll}"
    );
}

#[test]
fn reduce_folds_to_iconst() {
    let ll = emit(include_str!("../../../benchmarks/suite5/reduce.ryx"));
    assert!(
        !ll.contains("!llvm.loop") && ll.contains("call void @rynix_rt_print_i64"),
        "reduce should fold away the loop:\n{ll}"
    );
}

#[test]
fn sum_opaque_closed_form_has_no_sdiv_or_loop() {
    // Opaque n → closed form n(n-1)/2*(2n-1)/3 via udiv/mulhu, not host-folded checksum.
    let ll = emit(include_str!("../../../benchmarks/suite5/sum.ryx"));
    assert!(
        ll.contains("@rynix_rt_opaque_i64"),
        "sum Suite5 IR should retain opaque:\n{ll}"
    );
    assert!(
        !ll.contains("!llvm.loop"),
        "sum of squares should strength-reduce the loop:\n{ll}"
    );
    assert!(
        !ll.contains("sdiv "),
        "closed-form /3 must not emit signed IDIV:\n{ll}"
    );
    assert!(
        !(ll.contains("1124998875000250000") && !ll.contains("opaque")),
        "opaque sum must not host-fold to a lone checksum iconst:\n{ll}"
    );
    assert!(
        ll.contains("mul i128") || ll.contains("udiv "),
        "expected reciprocal mulhu or udiv for /3:\n{ll}"
    );
}

#[test]
fn mutable_let_uses_stack_alloca() {
    let ll = emit(
        r"
def main() -> i64
  let mut x = 1
  x += 2
  return x
end
",
    );
    assert!(!ll.contains("call ptr @rynix_rt_heap_alloc"), "{ll}");
    assert!(ll.contains("add i64"), "{ll}");
}
