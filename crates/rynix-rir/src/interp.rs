//! Tiny RIR interpreter — differential-testing oracle for later codegen.

#![allow(clippy::too_many_lines)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::manual_let_else)]

use rynix_span::Interner;
use rustc_hash::FxHashMap;

use crate::ir::{FuncId, Inst, IrTy, Module, ValueId};

#[derive(Clone, Debug, PartialEq)]
pub enum InterpValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Ptr(u32), // index into memory slots
    /// Heap buffer of i64 slots (array layout).
    Arr(u32),
    /// Pointer into an [`InterpValue::Arr`] buffer.
    Slot { arr: u32, idx: i64 },
}

#[derive(Debug)]
pub enum InterpError {
    MissingMain,
    Trap(String),
}

/// Execute `@main` with no arguments; return its value (or Unit).
pub fn interpret_module(
    module: &Module,
    interner: &Interner,
) -> Result<InterpValue, InterpError> {
    let main = module
        .func_names
        .iter()
        .position(|&n| interner.resolve(n) == "main")
        .map(|i| FuncId(i as u32))
        .ok_or(InterpError::MissingMain)?;
    let mut mem: Vec<InterpValue> = Vec::new();
    let mut arrays: Vec<Vec<i64>> = Vec::new();
    eval_func(module, interner, main, &[], &mut mem, &mut arrays)
}

fn eval_func(
    module: &Module,
    interner: &Interner,
    fid: FuncId,
    args: &[InterpValue],
    mem: &mut Vec<InterpValue>,
    arrays: &mut Vec<Vec<i64>>,
) -> Result<InterpValue, InterpError> {
    let func = module.func(fid);
    let mut vals: FxHashMap<ValueId, InterpValue> = FxHashMap::default();
    for (i, (vid, _)) in func.params.iter().enumerate() {
        vals.insert(*vid, args.get(i).cloned().unwrap_or(InterpValue::Unit));
    }

    let mut block = func.entry;
    loop {
        let b = func.block(block);
        for &(pid, _) in &b.params {
            // Block params should already be set by the branch.
            let _ = pid;
        }
        for &iid in &b.insts {
            let inst = func.inst(iid);
            let result_vid = func.values.iter().enumerate().find_map(|(vi, v)| {
                (v.def == Some(iid)).then_some(ValueId(vi as u32))
            });
            match inst {
                Inst::IConst(n) => {
                    vals.insert(result_vid.unwrap(), InterpValue::I64(*n));
                }
                Inst::FConst(n) => {
                    vals.insert(result_vid.unwrap(), InterpValue::F64(*n));
                }
                Inst::BConst(b) => {
                    vals.insert(result_vid.unwrap(), InterpValue::Bool(*b));
                }
                Inst::SConst(s) => {
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::Str(interner.resolve(*s).to_string()),
                    );
                }
                Inst::Nil => {
                    vals.insert(result_vid.unwrap(), InterpValue::Unit);
                }
                Inst::IAdd(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x + y)?);
                }
                Inst::ISub(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x - y)?);
                }
                Inst::IMul(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x * y)?);
                }
                Inst::IDiv(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x / y)?);
                }
                Inst::IRem(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x % y)?);
                }
                Inst::URem(a, b) => {
                    let InterpValue::I64(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("urem".into()));
                    };
                    let InterpValue::I64(y) = get(&vals, *b)? else {
                        return Err(InterpError::Trap("urem".into()));
                    };
                    if y <= 0 {
                        return Err(InterpError::Trap("urem div".into()));
                    }
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::I64((x as u64).rem_euclid(y as u64) as i64),
                    );
                }
                Inst::IAnd(a, b) => {
                    vals.insert(result_vid.unwrap(), iop(&vals, *a, *b, |x, y| x & y)?);
                }
                Inst::LShr(a, b) => {
                    let InterpValue::I64(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("lshr".into()));
                    };
                    let InterpValue::I64(y) = get(&vals, *b)? else {
                        return Err(InterpError::Trap("lshr".into()));
                    };
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::I64(((x as u64) >> (y as u32)) as i64),
                    );
                }
                Inst::LShl(a, b) => {
                    let InterpValue::I64(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("lshl".into()));
                    };
                    let InterpValue::I64(y) = get(&vals, *b)? else {
                        return Err(InterpError::Trap("lshl".into()));
                    };
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::I64(((x as u64) << (y as u32)) as i64),
                    );
                }
                Inst::INeg(a) => {
                    let InterpValue::I64(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("ineg".into()));
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::I64(-x));
                }
                Inst::ICmp(op, a, b) => {
                    let xa = get(&vals, *a)?;
                    let yb = get(&vals, *b)?;
                    let r = match (&xa, &yb) {
                        (InterpValue::I64(x), InterpValue::I64(y)) => match op {
                            crate::ir::CmpOp::Eq => x == y,
                            crate::ir::CmpOp::Ne => x != y,
                            crate::ir::CmpOp::Lt => x < y,
                            crate::ir::CmpOp::Le => x <= y,
                            crate::ir::CmpOp::Gt => x > y,
                            crate::ir::CmpOp::Ge => x >= y,
                        },
                        (InterpValue::Bool(x), InterpValue::Bool(y)) => match op {
                            crate::ir::CmpOp::Eq => x == y,
                            crate::ir::CmpOp::Ne => x != y,
                            _ => {
                                return Err(InterpError::Trap("icmp bool order".into()));
                            }
                        },
                        _ => return Err(InterpError::Trap("icmp".into())),
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::Bool(r));
                }
                Inst::BAnd(a, b) => {
                    let InterpValue::Bool(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("band".into()));
                    };
                    let InterpValue::Bool(y) = get(&vals, *b)? else {
                        return Err(InterpError::Trap("band".into()));
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::Bool(x && y));
                }
                Inst::BOr(a, b) => {
                    let InterpValue::Bool(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("bor".into()));
                    };
                    let InterpValue::Bool(y) = get(&vals, *b)? else {
                        return Err(InterpError::Trap("bor".into()));
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::Bool(x || y));
                }
                Inst::ZExtI64(a) => {
                    let InterpValue::Bool(b) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("zext_i64".into()));
                    };
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::I64(i64::from(b)),
                    );
                }
                Inst::CtPop(a) => {
                    let InterpValue::I64(n) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("ctpop".into()));
                    };
                    vals.insert(
                        result_vid.unwrap(),
                        InterpValue::I64(n.count_ones() as i64),
                    );
                }
                Inst::BNot(a) => {
                    let InterpValue::Bool(x) = get(&vals, *a)? else {
                        return Err(InterpError::Trap("bnot".into()));
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::Bool(!x));
                }
                Inst::Alloc { ty, .. } => {
                    let init = match ty {
                        IrTy::Bool => InterpValue::Bool(false),
                        IrTy::I64 => InterpValue::I64(0),
                        IrTy::F64 => InterpValue::F64(0.0),
                        IrTy::Str => InterpValue::Str(String::new()),
                        _ => InterpValue::Unit,
                    };
                    let idx = mem.len() as u32;
                    mem.push(init);
                    vals.insert(result_vid.unwrap(), InterpValue::Ptr(idx));
                }
                Inst::RegionCreate { .. } | Inst::RegionReset { .. } | Inst::Free { .. } => {
                    // Runtime markers — no-op in the oracle interpreter.
                }
                Inst::GepI64 { base, index } => {
                    let idx = match get(&vals, *index)? {
                        InterpValue::I64(n) => n,
                        _ => return Err(InterpError::Trap("gep index".into())),
                    };
                    let slot = match get(&vals, *base)? {
                        InterpValue::Arr(a) => InterpValue::Slot { arr: a, idx },
                        InterpValue::Slot { arr, idx: base_idx } => InterpValue::Slot {
                            arr,
                            idx: base_idx + idx,
                        },
                        _ => return Err(InterpError::Trap("gep base".into())),
                    };
                    vals.insert(result_vid.unwrap(), slot);
                }
                Inst::BoundsCheck { index, len } => {
                    let idx = match get(&vals, *index)? {
                        InterpValue::I64(n) => n,
                        _ => return Err(InterpError::Trap("bounds index".into())),
                    };
                    let ln = match get(&vals, *len)? {
                        InterpValue::I64(n) => n,
                        _ => return Err(InterpError::Trap("bounds len".into())),
                    };
                    if idx < 0 || idx >= ln {
                        return Err(InterpError::Trap("bounds check failed".into()));
                    }
                }
                Inst::ArrayLen(base) => {
                    let n = match get(&vals, *base)? {
                        InterpValue::Arr(a) => arrays[a as usize][0],
                        _ => return Err(InterpError::Trap("array_len".into())),
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::I64(n));
                }
                Inst::LoadIndex { base, index } => {
                    let idx = match get(&vals, *index)? {
                        InterpValue::I64(n) => n,
                        _ => return Err(InterpError::Trap("load_index".into())),
                    };
                    let n = match get(&vals, *base)? {
                        InterpValue::Arr(a) => arrays[a as usize][(idx + 1) as usize],
                        _ => return Err(InterpError::Trap("load_index base".into())),
                    };
                    vals.insert(result_vid.unwrap(), InterpValue::I64(n));
                }
                Inst::Load(p) => match get(&vals, *p)? {
                    InterpValue::Ptr(idx) => {
                        vals.insert(result_vid.unwrap(), mem[idx as usize].clone());
                    }
                    InterpValue::Slot { arr, idx } => {
                        vals.insert(
                            result_vid.unwrap(),
                            InterpValue::I64(arrays[arr as usize][idx as usize]),
                        );
                    }
                    _ => return Err(InterpError::Trap("load".into())),
                },
                Inst::Store { ptr, value } => match get(&vals, *ptr)? {
                    InterpValue::Ptr(idx) => {
                        mem[idx as usize] = get(&vals, *value)?;
                    }
                    InterpValue::Slot { arr, idx } => {
                        let v = match get(&vals, *value)? {
                            InterpValue::I64(n) => n,
                            InterpValue::Bool(b) => i64::from(b),
                            _ => return Err(InterpError::Trap("store value".into())),
                        };
                        arrays[arr as usize][idx as usize] = v;
                    }
                    _ => return Err(InterpError::Trap("store".into())),
                },
                Inst::Call { func, args } => {
                    let argv: Result<Vec<_>, _> =
                        args.iter().map(|a| get(&vals, *a)).collect();
                    let argv = argv?;
                    let ret = eval_func(module, interner, *func, &argv, mem, arrays)?;
                    if let Some(r) = result_vid {
                        vals.insert(r, ret);
                    }
                }
                Inst::CallExt { name, args, ret } => {
                    let n = interner.resolve(*name);
                    if n == "print" || n == "rynix_rt_print_i64" || n == "print_i64" {
                        for a in args {
                            let _ = get(&vals, *a)?;
                        }
                    }
                    if n == "rynix_rt_heap_alloc" {
                        let size = match args.first().map(|a| get(&vals, *a)).transpose()? {
                            Some(InterpValue::I64(s)) => s,
                            _ => 0,
                        };
                        let slots = (size / 8).max(1) as usize;
                        let id = arrays.len() as u32;
                        arrays.push(vec![0; slots]);
                        if let Some(r) = result_vid {
                            vals.insert(r, InterpValue::Arr(id));
                        }
                    } else if let Some(r) = result_vid {
                        vals.insert(
                            r,
                            match ret {
                                IrTy::I64 => InterpValue::I64(0),
                                IrTy::Bool => InterpValue::Bool(false),
                                IrTy::Unit => InterpValue::Unit,
                                _ => InterpValue::Unit,
                            },
                        );
                    }
                }
                Inst::Ret(None) => return Ok(InterpValue::Unit),
                Inst::Ret(Some(v)) => return get(&vals, *v),
                Inst::Jump { target, args } => {
                    let bparams = &func.block(*target).params;
                    for (i, (pid, _)) in bparams.iter().enumerate() {
                        if let Some(a) = args.get(i) {
                            vals.insert(*pid, get(&vals, *a)?);
                        }
                    }
                    block = *target;
                    break;
                }
                Inst::Br {
                    cond,
                    then_target,
                    then_args,
                    else_target,
                    else_args,
                } => {
                    let InterpValue::Bool(c) = get(&vals, *cond)? else {
                        return Err(InterpError::Trap("br cond".into()));
                    };
                    let (target, args) = if c {
                        (*then_target, then_args)
                    } else {
                        (*else_target, else_args)
                    };
                    let bparams = &func.block(target).params;
                    for (i, (pid, _)) in bparams.iter().enumerate() {
                        if let Some(a) = args.get(i) {
                            vals.insert(*pid, get(&vals, *a)?);
                        }
                    }
                    block = target;
                    break;
                }
                Inst::Unreachable => return Err(InterpError::Trap("unreachable".into())),
                _ => {
                    // Floats etc. — skip / zero.
                    if let Some(r) = result_vid {
                        vals.insert(r, InterpValue::I64(0));
                    }
                }
            }
            // If terminator handled via Jump/Br, inner break only exits inst loop —
            // we use a flag. Actually Jump/Br use `break` from the for-loop, then
            // the outer loop continues with new block. Good.
            if inst.is_terminator() && !matches!(inst, Inst::Jump { .. } | Inst::Br { .. }) {
                // Ret / Unreachable already returned.
            }
        }
    }
}

fn get(
    vals: &FxHashMap<ValueId, InterpValue>,
    id: ValueId,
) -> Result<InterpValue, InterpError> {
    vals.get(&id)
        .cloned()
        .ok_or_else(|| InterpError::Trap(format!("undefined %{}", id.0)))
}

fn iop(
    vals: &FxHashMap<ValueId, InterpValue>,
    a: ValueId,
    b: ValueId,
    f: impl Fn(i64, i64) -> i64,
) -> Result<InterpValue, InterpError> {
    let InterpValue::I64(x) = get(vals, a)? else {
        return Err(InterpError::Trap("expected i64".into()));
    };
    let InterpValue::I64(y) = get(vals, b)? else {
        return Err(InterpError::Trap("expected i64".into()));
    };
    Ok(InterpValue::I64(f(x, y)))
}
