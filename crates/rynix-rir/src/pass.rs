//! Baseline RIR passes: DCE, const-fold, simplify-cfg.

#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::many_single_char_names)]

use rustc_hash::FxHashSet;

use crate::ir::{Inst, Module, ValueId};
use crate::verify::verify_module;

/// Run the standard Phase-5 pipeline. Returns verifier errors if any remain.
pub fn run_pipeline(module: &mut Module) -> Vec<String> {
    const_fold(module);
    simplify_cfg(module);
    dce(module);
    verify_module(module)
}

/// Fold obvious constant arithmetic in place.
pub fn const_fold(module: &mut Module) {
    for func in &mut module.funcs {
        let mut known: Vec<Option<i64>> = vec![None; func.values.len()];
        let mut rewrites: Vec<(usize, i64)> = Vec::new();
        for (ii, inst) in func.insts.iter().enumerate() {
            let result = func
                .values
                .iter()
                .enumerate()
                .find(|(_, v)| v.def == Some(crate::ir::InstId(ii as u32)))
                .map(|(i, _)| i);
            let folded = match *inst {
                Inst::IConst(n) => Some(n),
                Inst::BConst(b) => Some(i64::from(b)),
                Inst::IAdd(a, b) => bin(&known, a, b, |x, y| x.wrapping_add(y)),
                Inst::ISub(a, b) => bin(&known, a, b, |x, y| x.wrapping_sub(y)),
                Inst::IMul(a, b) => bin(&known, a, b, |x, y| x.wrapping_mul(y)),
                Inst::INeg(a) => known.get(a.0 as usize).copied().flatten().map(|x| -x),
                _ => None,
            };
            if let (Some(ri), Some(n)) = (result, folded) {
                known[ri] = Some(n);
                if !matches!(inst, Inst::IConst(_)) {
                    rewrites.push((ii, n));
                }
            }
        }
        for (ii, n) in rewrites {
            func.insts[ii] = Inst::IConst(n);
        }
    }
}

fn bin(known: &[Option<i64>], a: ValueId, b: ValueId, f: impl Fn(i64, i64) -> i64) -> Option<i64> {
    let x = known.get(a.0 as usize).copied().flatten()?;
    let y = known.get(b.0 as usize).copied().flatten()?;
    Some(f(x, y))
}

/// Remove instructions whose results are never used (non-side-effecting).
pub fn dce(module: &mut Module) {
    for func in &mut module.funcs {
        let mut used = FxHashSet::default();
        // Terminators and stores/calls are roots; also ret values.
        for inst in &func.insts {
            mark_used(inst, &mut used);
        }
        // Block params always live.
        for block in &func.blocks {
            for (v, _) in &block.params {
                used.insert(*v);
            }
        }
        for (p, _) in &func.params {
            used.insert(*p);
        }

        // Iterate to fixed point: if an inst's result is used, its operands are used.
        let mut changed = true;
        while changed {
            changed = false;
            for (ii, inst) in func.insts.iter().enumerate() {
                let result = func.values.iter().enumerate().find_map(|(vi, v)| {
                    (v.def == Some(crate::ir::InstId(ii as u32))).then_some(ValueId(vi as u32))
                });
                let live = match result {
                    Some(v) => used.contains(&v),
                    None => true, // side-effecting / terminators keep operands
                };
                if live || is_effectful(inst) {
                    let before = used.len();
                    mark_used(inst, &mut used);
                    if used.len() != before {
                        changed = true;
                    }
                }
            }
        }

        // Replace dead pure insts with nop-like iconst 0 (keep ids stable).
        for (ii, inst) in func.insts.iter_mut().enumerate() {
            if is_effectful(inst) || inst.is_terminator() {
                continue;
            }
            let result = func.values.iter().enumerate().find_map(|(vi, v)| {
                (v.def == Some(crate::ir::InstId(ii as u32))).then_some(ValueId(vi as u32))
            });
            if let Some(v) = result
                && !used.contains(&v)
            {
                *inst = Inst::IConst(0);
            }
        }
    }
}

fn is_effectful(inst: &Inst) -> bool {
    matches!(
        inst,
        Inst::Store { .. }
            | Inst::Call { .. }
            | Inst::CallExt { .. }
            | Inst::RegionCreate { .. }
            | Inst::RegionReset { .. }
            | Inst::Free { .. }
            | Inst::Ret(_)
            | Inst::Jump { .. }
            | Inst::Br { .. }
            | Inst::Unreachable
            | Inst::Alloc { .. } // keep alloc sites for Phase 6
    )
}

fn mark_used(inst: &Inst, used: &mut FxHashSet<ValueId>) {
    match inst {
        Inst::IAdd(a, b)
        | Inst::ISub(a, b)
        | Inst::IMul(a, b)
        | Inst::IDiv(a, b)
        | Inst::IRem(a, b)
        | Inst::FAdd(a, b)
        | Inst::FSub(a, b)
        | Inst::FMul(a, b)
        | Inst::FDiv(a, b)
        | Inst::ICmp(_, a, b)
        | Inst::FCmp(_, a, b)
        | Inst::Store { ptr: a, value: b } => {
            used.insert(*a);
            used.insert(*b);
        }
        Inst::INeg(a) | Inst::FNeg(a) | Inst::BNot(a) | Inst::Load(a) | Inst::Ret(Some(a)) => {
            used.insert(*a);
        }
        Inst::Call { args, .. } | Inst::CallExt { args, .. } => {
            for a in args {
                used.insert(*a);
            }
        }
        Inst::Jump { args, .. } => {
            for a in args {
                used.insert(*a);
            }
        }
        Inst::Br {
            cond,
            then_args,
            else_args,
            ..
        } => {
            used.insert(*cond);
            for a in then_args.iter().chain(else_args.iter()) {
                used.insert(*a);
            }
        }
        _ => {}
    }
}

/// Remove trivial empty jump chains: `jump b` where b only jumps to c → jump c.
pub fn simplify_cfg(module: &mut Module) {
    for func in &mut module.funcs {
        // Collect redirect: block → ultimate target if it is a pure jump with no params.
        let mut redirect = vec![None; func.blocks.len()];
        for (bi, block) in func.blocks.iter().enumerate() {
            if block.params.is_empty()
                && block.insts.len() == 1
                && let Inst::Jump { target, args } = func.inst(block.insts[0])
                && args.is_empty()
            {
                redirect[bi] = Some(*target);
            }
        }
        // Apply redirects to terminators.
        for inst in &mut func.insts {
            match inst {
                Inst::Jump { target, .. } => {
                    while let Some(next) = redirect[target.0 as usize] {
                        *target = next;
                    }
                }
                Inst::Br {
                    then_target,
                    else_target,
                    ..
                } => {
                    while let Some(next) = redirect[then_target.0 as usize] {
                        *then_target = next;
                    }
                    while let Some(next) = redirect[else_target.0 as usize] {
                        *else_target = next;
                    }
                }
                _ => {}
            }
        }
    }
}
