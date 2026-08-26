//! Reject dangerous external CallExt names before emit (ADR-0023).

use rynix_span::Interner;

use crate::ir::{Inst, Module};

/// Names that must never appear as `CallExt` callees (process / dynamic load escapes).
pub fn is_dangerous_ext_name(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    let base = base.strip_prefix('_').unwrap_or(base);
    matches!(
        base,
        "system"
            | "exec"
            | "execl"
            | "execle"
            | "execlp"
            | "execv"
            | "execve"
            | "execvp"
            | "execvpe"
            | "popen"
            | "dlopen"
            | "rynix_rt_system"
            | "rynix_rt_exec"
            | "rynix_rt_popen"
            | "rynix_rt_dlopen"
    ) || base.starts_with("exec") && base != "execute" // catch exec* family; allow unrelated "execute"
}

/// Scan a module for denylisted `CallExt` names. Returns human-readable errors.
pub fn sanitize_module(module: &Module, interner: &Interner) -> Vec<String> {
    let mut errs = Vec::new();
    for (fi, func) in module.funcs.iter().enumerate() {
        let fname = module
            .func_names
            .get(fi)
            .map(|s| interner.resolve(*s))
            .unwrap_or("<fn>");
        for inst in &func.insts {
            if let Inst::CallExt { name, .. } = inst {
                let n = interner.resolve(*name);
                if is_dangerous_ext_name(n) {
                    errs.push(format!(
                        "sanitize: rejected dangerous CallExt `{n}` in `{fname}` \
                         (system/exec*/popen/dlopen escapes are not allowed)"
                    ));
                }
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionBuilder, IrTy, Module};
    use rynix_span::Interner;

    #[test]
    fn rejects_system_call_ext() {
        let mut interner = Interner::new();
        let main = interner.intern("main");
        let system = interner.intern("system");
        let mut b = FunctionBuilder::new(main, IrTy::I64);
        let arg = b.iconst(0);
        let _ = b.call_ext(system, vec![arg], IrTy::I64);
        let zero = b.iconst(0);
        b.ret(Some(zero));
        let mut module = Module::new();
        module.func_names.push(main);
        module.funcs.push(b.finish());
        let errs = sanitize_module(&module, &interner);
        assert!(!errs.is_empty(), "expected sanitize errors");
        assert!(
            errs.iter().any(|e| e.contains("system") && e.contains("sanitize")),
            "{errs:?}"
        );
    }

    #[test]
    fn allows_print_i64() {
        let mut interner = Interner::new();
        let main = interner.intern("main");
        let print = interner.intern("print_i64");
        let mut b = FunctionBuilder::new(main, IrTy::I64);
        let arg = b.iconst(1);
        let _ = b.call_ext(print, vec![arg], IrTy::Unit);
        let zero = b.iconst(0);
        b.ret(Some(zero));
        let mut module = Module::new();
        module.func_names.push(main);
        module.funcs.push(b.finish());
        assert!(sanitize_module(&module, &interner).is_empty());
    }

    #[test]
    fn denylist_helpers() {
        assert!(is_dangerous_ext_name("system"));
        assert!(is_dangerous_ext_name("execve"));
        assert!(is_dangerous_ext_name("popen"));
        assert!(is_dangerous_ext_name("dlopen"));
        assert!(is_dangerous_ext_name("rynix_rt_system"));
        assert!(!is_dangerous_ext_name("print_i64"));
        assert!(!is_dangerous_ext_name("execute"));
    }
}
