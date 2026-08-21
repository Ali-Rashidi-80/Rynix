//! Unit tests for escape lattice extremes built via `FunctionBuilder`.

use rynix_rir::{
    analyze_escape, inject_regions, verify_module, Escape, FunctionBuilder, Inst, IrTy, Module,
    Placement,
};
use rynix_span::{Interner, Span};

#[test]
fn local_slot_is_stack() {
    let mut interner = Interner::new();
    let name = interner.intern("main");
    let mut b = FunctionBuilder::new(name, IrTy::I64);
    let slot = b.alloc(IrTy::I64, Span::empty(0));
    let v = b.iconst(1);
    b.store(slot, v);
    let loaded = b.load(slot);
    b.ret(Some(loaded));
    let mut module = Module::new();
    module.func_names.push(name);
    module.funcs.push(b.finish());

    let report = analyze_escape(&module, &interner);
    assert_eq!(report.sites.len(), 1);
    assert_eq!(report.sites[0].escape, Escape::NoEscape);
    assert_eq!(report.sites[0].placement, Placement::Stack);
}

#[test]
fn call_ext_unknown_makes_heap() {
    let mut interner = Interner::new();
    let name = interner.intern("main");
    let ext = interner.intern("retain");
    let mut b = FunctionBuilder::new(name, IrTy::Unit);
    let slot = b.alloc(IrTy::I64, Span::empty(10));
    let _ = b.call_ext(ext, vec![slot], IrTy::Unit);
    b.ret(None);
    let mut module = Module::new();
    module.func_names.push(name);
    module.funcs.push(b.finish());

    let report = analyze_escape(&module, &interner);
    assert_eq!(report.sites[0].escape, Escape::GlobalEscape);
    assert_eq!(report.sites[0].placement, Placement::Heap);
    assert!(report.sites[0].free_at.is_some());

    inject_regions(&mut module, &report);
    let text = format!("{:?}", module.funcs[0].insts);
    assert!(
        module.funcs[0]
            .insts
            .iter()
            .any(|i| matches!(i, Inst::Free { .. })),
        "expected free injection: {text}"
    );
    assert!(verify_module(&module).is_empty());
}

#[test]
fn returned_pointer_is_arg_escape_region() {
    let mut interner = Interner::new();
    let name = interner.intern("make");
    let mut b = FunctionBuilder::new(name, IrTy::Ptr);
    let slot = b.alloc(IrTy::I64, Span::empty(20));
    b.ret(Some(slot));
    let mut module = Module::new();
    module.func_names.push(name);
    module.funcs.push(b.finish());

    let report = analyze_escape(&module, &interner);
    assert_eq!(report.sites[0].escape, Escape::ArgEscape);
    assert!(matches!(report.sites[0].placement, Placement::Region(_)));

    inject_regions(&mut module, &report);
    assert!(
        module.funcs[0]
            .insts
            .iter()
            .any(|i| matches!(i, Inst::RegionCreate { .. })),
        "expected region_create"
    );
}

#[test]
fn print_is_benign() {
    let mut interner = Interner::new();
    let name = interner.intern("main");
    let print = interner.intern("print");
    let mut b = FunctionBuilder::new(name, IrTy::Unit);
    let slot = b.alloc(IrTy::Str, Span::empty(0));
    let _ = b.call_ext(print, vec![slot], IrTy::Unit);
    b.ret(None);
    let mut module = Module::new();
    module.func_names.push(name);
    module.funcs.push(b.finish());

    let report = analyze_escape(&module, &interner);
    assert_eq!(report.sites[0].escape, Escape::NoEscape);
}
