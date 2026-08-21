//! Interval / Presburger-lite bounds-check elimination.
//!
//! Tracks concrete `i64` constants and removes [`Inst::BoundsCheck`] when
//! `0 <= index < len` is proven at the check site. Also recovers array
//! lengths from `store gep(base,0), iconst N` followed by `array_len base`.

use rustc_hash::FxHashMap;

use crate::ir::{Inst, InstId, Module, ValueId};

/// Eliminate provably redundant bounds checks in place.
pub fn eliminate_bounds_checks(module: &mut Module) {
    for func in &mut module.funcs {
        let mut known: FxHashMap<ValueId, i64> = FxHashMap::default();
        let mut array_len: FxHashMap<ValueId, i64> = FxHashMap::default();
        let mut eliminated = vec![false; func.insts.len()];

        for (ii, inst) in func.insts.iter().enumerate() {
            let result = func
                .values
                .iter()
                .enumerate()
                .find(|(_, v)| v.def == Some(InstId(ii as u32)))
                .map(|(i, _)| ValueId(i as u32));

            match *inst {
                Inst::IConst(n) => {
                    if let Some(r) = result {
                        known.insert(r, n);
                    }
                }
                Inst::BConst(b) => {
                    if let Some(r) = result {
                        known.insert(r, i64::from(b));
                    }
                }
                Inst::IAdd(a, b) => fold_bin(&mut known, result, a, b, i64::wrapping_add),
                Inst::ISub(a, b) => fold_bin(&mut known, result, a, b, i64::wrapping_sub),
                Inst::IMul(a, b) => fold_bin(&mut known, result, a, b, i64::wrapping_mul),
                Inst::INeg(a) => {
                    if let (Some(r), Some(x)) = (result, known.get(&a).copied()) {
                        known.insert(r, -x);
                    }
                }
                Inst::GepI64 { base, index } => {
                    if let (Some(r), Some(0)) = (result, known.get(&index).copied()) {
                        // Mark this gep as the length slot of `base`.
                        // Stored in known as a sentinel via a side table keyed by gep result.
                        let _ = (r, base);
                        // Use negative map: length_slot[gep] = base
                        // handled in Store below by checking Gep def.
                    }
                }
                Inst::Store { ptr, value } => {
                    if let Some(len) = known.get(&value).copied() {
                        // If ptr is gep(base, 0), record array_len[base]=len.
                        if let Some(def) = func.value(ptr).def
                            && let Inst::GepI64 { base, index } = func.inst(def)
                            && known.get(index).copied() == Some(0)
                        {
                            array_len.insert(*base, len);
                        }
                    }
                }
                Inst::ArrayLen(base) => {
                    if let (Some(r), Some(n)) = (result, array_len.get(&base).copied()) {
                        known.insert(r, n);
                    }
                }
                Inst::BoundsCheck { index, len } => {
                    if let (Some(idx), Some(ln)) =
                        (known.get(&index).copied(), known.get(&len).copied())
                        && idx >= 0
                        && idx < ln
                    {
                        eliminated[ii] = true;
                    }
                }
                _ => {}
            }
        }

        for block in &mut func.blocks {
            block.insts.retain(|&iid| !eliminated[iid.0 as usize]);
        }
        for (ii, flag) in eliminated.iter().enumerate() {
            if *flag {
                func.insts[ii] = Inst::IConst(0);
            }
        }
    }
}

fn fold_bin(
    known: &mut FxHashMap<ValueId, i64>,
    result: Option<ValueId>,
    a: ValueId,
    b: ValueId,
    op: fn(i64, i64) -> i64,
) {
    if let (Some(r), Some(x), Some(y)) = (result, known.get(&a).copied(), known.get(&b).copied()) {
        known.insert(r, op(x, y));
    }
}
