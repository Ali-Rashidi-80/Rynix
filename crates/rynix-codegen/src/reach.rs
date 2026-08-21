//! Whole-program reachability from `main` (RIR-level DCE).

use rynix_rir::{FuncId, Inst, Module};
use rynix_span::Interner;
use rustc_hash::FxHashSet;

/// Function ids reachable from `@main` (empty if `main` is missing).
pub fn reachable_from_main(module: &Module, interner: &Interner) -> FxHashSet<FuncId> {
    let Some(main) = module
        .func_names
        .iter()
        .position(|&n| interner.resolve(n) == "main")
        .map(|i| FuncId(i as u32))
    else {
        return FxHashSet::default();
    };

    let mut seen = FxHashSet::default();
    let mut stack = vec![main];
    while let Some(fid) = stack.pop() {
        if !seen.insert(fid) {
            continue;
        }
        for inst in &module.func(fid).insts {
            if let Inst::Call { func, .. } = inst {
                stack.push(*func);
            }
        }
    }
    seen
}

/// Drop functions not reachable from `main`. Remaps [`FuncId`]s in calls.
pub fn prune_unreachable(module: &mut Module, interner: &Interner) {
    let keep = reachable_from_main(module, interner);
    if keep.is_empty() {
        return;
    }

    let old_funcs = std::mem::take(&mut module.funcs);
    let old_names = std::mem::take(&mut module.func_names);
    let mut old_to_new: Vec<Option<FuncId>> = vec![None; old_funcs.len()];
    let mut new_funcs = Vec::new();
    let mut new_names = Vec::new();

    for (i, func) in old_funcs.into_iter().enumerate() {
        let old = FuncId(i as u32);
        if keep.contains(&old) {
            let neu = FuncId(new_funcs.len() as u32);
            old_to_new[i] = Some(neu);
            new_funcs.push(func);
            new_names.push(old_names[i]);
        }
    }
    module.funcs = new_funcs;
    module.func_names = new_names;

    for func in &mut module.funcs {
        for inst in &mut func.insts {
            if let Inst::Call { func: callee, .. } = inst
                && let Some(neu) = old_to_new[callee.0 as usize]
            {
                *callee = neu;
            }
        }
    }
}
