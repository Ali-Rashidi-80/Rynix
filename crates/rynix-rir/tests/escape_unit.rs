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

/// Mutual recursion SCC: allocator returns a pointer; peer calls back into the
/// allocator. After SCC fixpoint the site must not stay `NoEscape`.
#[test]
fn interproc_scc_mutual_recursion_arg_escape() {
    use rynix_rir::FuncId;

    let mut interner = Interner::new();
    let na = interner.intern("a");
    let nb = interner.intern("b");

    // a(): ptr  — alloc; call b(); ret slot
    let mut ba = FunctionBuilder::new(na, IrTy::Ptr);
    let slot = ba.alloc(IrTy::I64, Span::empty(1));
    let _ = ba.call(FuncId(1), vec![], IrTy::Ptr);
    ba.ret(Some(slot));

    // b(): ptr  — call a(); ret result
    let mut bb = FunctionBuilder::new(nb, IrTy::Ptr);
    let r = bb.call(FuncId(0), vec![], IrTy::Ptr);
    bb.ret(Some(r));

    let mut module = Module::new();
    module.func_names.push(na);
    module.func_names.push(nb);
    module.funcs.push(ba.finish());
    module.funcs.push(bb.finish());

    let report = analyze_escape(&module, &interner);
    assert!(
        !report.sites.is_empty(),
        "expected at least one alloc site"
    );
    assert!(
        report
            .sites
            .iter()
            .any(|s| s.escape != Escape::NoEscape),
        "SCC fixpoint must promote escape beyond NoEscape; got {:?}",
        report.sites.iter().map(|s| s.escape).collect::<Vec<_>>()
    );
}
