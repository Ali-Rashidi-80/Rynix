//! Lightweight structural verifier for RIR functions.

use crate::ir::{Inst, Module};

/// Verify every function in `module`. Returns human-readable problems.
pub fn verify_module(module: &Module) -> Vec<String> {
    let mut errors = Vec::new();
    for (fi, func) in module.funcs.iter().enumerate() {
        for (bi, block) in func.blocks.iter().enumerate() {
            if block.insts.is_empty() {
                errors.push(format!("func#{fi} block{bi}: empty block"));
                continue;
            }
            let last = *block.insts.last().unwrap();
            if !func.inst(last).is_terminator() {
                errors.push(format!(
                    "func#{fi} block{bi}: block does not end in a terminator"
                ));
            }
            for &iid in &block.insts[..block.insts.len().saturating_sub(1)] {
                if func.inst(iid).is_terminator() {
                    errors.push(format!(
                        "func#{fi} block{bi}: terminator in the middle of the block"
                    ));
                }
            }
            for &iid in &block.insts {
                check_inst(module, fi, bi, iid.0, func.inst(iid), &mut errors);
            }
        }
    }
    errors
}

fn check_inst(
    module: &Module,
    fi: usize,
    bi: usize,
    iid: u32,
    inst: &Inst,
    errors: &mut Vec<String>,
) {
    let nvals = module.funcs[fi].values.len() as u32;
    let nblocks = module.funcs[fi].blocks.len() as u32;
    let check_v = |v: crate::ir::ValueId, errors: &mut Vec<String>| {
        if v.0 >= nvals {
            errors.push(format!("func#{fi} block{bi} inst{iid}: bad value %{}", v.0));
        }
    };
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
        | Inst::Store { ptr: a, value: b }
        | Inst::GepI64 { base: a, index: b }
        | Inst::BoundsCheck { index: a, len: b }
        | Inst::LoadIndex { base: a, index: b } => {
            check_v(*a, errors);
            check_v(*b, errors);
        }
        Inst::INeg(a)
        | Inst::FNeg(a)
        | Inst::BNot(a)
        | Inst::Load(a)
        | Inst::ArrayLen(a)
        | Inst::Ret(Some(a))
        | Inst::Free { ptr: a, .. } => {
            check_v(*a, errors);
        }
        Inst::Call { func, args } => {
            if func.0 as usize >= module.funcs.len() {
                errors.push(format!("func#{fi} block{bi} inst{iid}: bad callee"));
            }
            for a in args {
                check_v(*a, errors);
            }
        }
        Inst::CallExt { args, .. } => {
            for a in args {
                check_v(*a, errors);
            }
        }
        Inst::Jump { target, args } => {
            if target.0 >= nblocks {
                errors.push(format!("func#{fi} block{bi} inst{iid}: bad jump target"));
            }
            for a in args {
                check_v(*a, errors);
            }
        }
        Inst::Br {
            cond,
            then_target,
            then_args,
            else_target,
            else_args,
        } => {
            check_v(*cond, errors);
            if then_target.0 >= nblocks || else_target.0 >= nblocks {
                errors.push(format!("func#{fi} block{bi} inst{iid}: bad br target"));
            }
            for a in then_args.iter().chain(else_args.iter()) {
                check_v(*a, errors);
            }
        }
        _ => {}
    }
}
