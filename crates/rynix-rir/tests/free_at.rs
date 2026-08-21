//! `#^ free-at` — heap sites must get an injected `Free`.

use rynix_rir::{analyze_escape, inject_regions, Escape, FunctionBuilder, Inst, IrTy, Module};
use rynix_span::{Interner, Span};

#[test]
fn unknown_call_ext_gets_free() {
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
    assert!(report.sites[0].free_at.is_some(), "#^ free-at site recorded");
    inject_regions(&mut module, &report);
    let has_free = module.funcs[0]
        .insts
        .iter()
        .any(|i| matches!(i, Inst::Free { .. }));
    assert!(has_free, "expected Free after inject_regions");
}
