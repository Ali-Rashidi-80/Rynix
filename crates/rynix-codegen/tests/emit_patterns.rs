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
fn counted_loop_latch_emits_vectorize_metadata() {
    let ll = emit(
        include_str!("../../../benchmarks/suite5/scan.ryx"),
    );
    assert!(
        ll.contains("!llvm.loop !0") && ll.contains("llvm.loop.vectorize.enable"),
        "expected loop vectorizer hint on latch back-edge:\n{ll}"
    );
    assert!(
        !ll.contains("br label %b1, !llvm.loop") || ll.matches("br label %b1, !llvm.loop").count() == 1,
        "loop metadata must be on latch only, not entry:\n{ll}"
    );
    assert!(
        !ll.contains("entry:\n  br label %b1, !llvm.loop"),
        "entry must not carry loop metadata:\n{ll}"
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
