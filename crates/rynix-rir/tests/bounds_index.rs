//! Index / bounds-check / BCE smoke tests.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{
    eliminate_bounds_checks, interpret_module, lower_module, print_module, run_pipeline,
    FunctionBuilder, Inst, IrTy, Module,
};
use rynix_sema::analyze;
use rynix_span::{Interner, Span};

fn lower(src: &str) -> (rynix_rir::Module, Interner) {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let rir = lower_module(module, &analysis, &mut interner, src, 0);
    (rir, interner)
}

#[test]
fn index_lowers_with_bounds_check() {
    let src = r"
def main() -> i64
  let a = [10, 20, 30]
  return a[1]
end
";
    let (rir, interner) = lower(src);
    let text = print_module(&rir, &interner);
    assert!(text.contains("bounds_check"), "{text}");
    assert!(text.contains("load_index"), "{text}");
    let v = interpret_module(&rir, &interner).expect("interp");
    assert_eq!(v, rynix_rir::InterpValue::I64(20));
}

#[test]
fn bce_removes_constant_in_range_check() {
    // Hand-built: bounds_check 1, 3 must vanish.
    let mut interner = Interner::new();
    let name = interner.intern("main");
    let mut module = Module::new();
    let mut b = FunctionBuilder::new(name, IrTy::I64);
    let idx = b.iconst(1);
    let len = b.iconst(3);
    b.push(Inst::BoundsCheck { index: idx, len });
    b.ret(Some(idx));
    module.func_names.push(name);
    module.funcs.push(b.finish());

    let before = module.funcs[0]
        .blocks
        .iter()
        .flat_map(|blk| {
            blk.insts
                .iter()
                .map(|&id| &module.funcs[0].insts[id.0 as usize])
        })
        .filter(|i| matches!(i, Inst::BoundsCheck { .. }))
        .count();
    assert_eq!(before, 1);
    eliminate_bounds_checks(&mut module);
    let after = module.funcs[0]
        .blocks
        .iter()
        .flat_map(|blk| {
            blk.insts
                .iter()
                .map(|&id| &module.funcs[0].insts[id.0 as usize])
        })
        .filter(|i| matches!(i, Inst::BoundsCheck { .. }))
        .count();
    assert_eq!(after, 0);
    let _ = Span::empty(0);
}

#[test]
fn pipeline_still_verifies_with_arrays() {
    let src = r"
def main() -> i64
  let a = [7]
  return a[0]
end
";
    let (mut rir, _) = lower(src);
    let errs = run_pipeline(&mut rir);
    assert!(errs.is_empty(), "{errs:?}");
}
