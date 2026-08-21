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
fn add_uses_stack_alloca_not_heap() {
    let ll = emit(
        r"
def main() -> i64
  let x = 1
  let y = 2
  return x + y
end
",
    );
    assert!(ll.contains("alloca"), "{ll}");
    assert!(!ll.contains("call ptr @rynix_rt_heap_alloc"), "{ll}");
    assert!(ll.contains("add i64"), "{ll}");
}
