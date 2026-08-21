//! Inject `region_create` / `region_reset` and annotate heap frees.

use rustc_hash::FxHashSet;

use crate::ir::{AllocSite, BlockId, FuncId, Inst, IrTy, Module};

use super::analyze::EscapeReport;
use super::lattice::{Escape, Placement};

/// Rewrite the module in place:
/// - insert `RegionCreate` at function entry when any site is region-placed;
/// - insert `RegionReset` at loop headers that dominate region allocs (v0: every
///   block that jumps to itself or is a trivial loop header);
/// - insert `Free` after the last use of each heap site when known.
pub fn inject_regions(module: &mut Module, report: &EscapeReport) {
    for (fi, func) in module.funcs.iter_mut().enumerate() {
        let fid = FuncId(fi as u32);
        let sites: Vec<_> = report.sites_for_func(fid).cloned().collect();
        if sites.is_empty() {
            continue;
        }

        let needs_region = sites.iter().any(|s| matches!(s.placement, Placement::Region(_)));
        let region_id = 0u32;

        // Annotate alloc instructions with placement (via rewriting Alloc — placement
        // is carried in report; we insert region markers and frees).
        if needs_region {
            insert_at_block_start(func, func.entry, Inst::RegionCreate { region: region_id });
        }

        // Loop headers: blocks that appear as Jump/Br targets of themselves (back-edges).
        let mut headers = FxHashSet::default();
        for block in &func.blocks {
            for &iid in &block.insts {
                match func.inst(iid) {
                    Inst::Jump { target, .. } => {
                        // Heuristic: if target has a back-edge from a later block, treat as header.
                        let _ = target;
                    }
                    Inst::Br {
                        then_target,
                        else_target,
                        ..
                    } => {
                        let _ = (then_target, else_target);
                    }
                    _ => {}
                }
            }
        }
        // Detect self-loops and simple back-edges: jump to block with smaller index from larger.
        for (bi, block) in func.blocks.iter().enumerate() {
            if let Some(&last) = block.insts.last() {
                match func.inst(last) {
                    Inst::Jump { target, .. } if target.0 as usize <= bi => {
                        headers.insert(*target);
                    }
                    Inst::Br {
                        then_target,
                        else_target,
                        ..
                    } => {
                        if then_target.0 as usize <= bi {
                            headers.insert(*then_target);
                        }
                        if else_target.0 as usize <= bi {
                            headers.insert(*else_target);
                        }
                    }
                    _ => {}
                }
            }
        }

        if needs_region {
            let headers: Vec<BlockId> = headers.into_iter().collect();
            for h in headers {
                if h != func.entry {
                    insert_at_block_start(func, h, Inst::RegionReset { region: region_id });
                }
            }
        }

        // Heap frees: insert Free after last-use instruction.
        let mut frees: Vec<(u32, AllocSite)> = sites
            .iter()
            .filter(|s| s.escape == Escape::GlobalEscape)
            .filter_map(|s| s.free_at.map(|at| (at, s.site)))
            .collect();
        frees.sort_by_key(|(at, _)| *at);
        // Insert from the end so indices stay valid.
        for (at, site) in frees.into_iter().rev() {
            if let Some(ptr) = alloc_ptr_for_site(func, site) {
                insert_after_inst(func, at, Inst::Free { site, ptr });
            }
        }
    }
}

fn alloc_ptr_for_site(func: &crate::ir::Function, site: AllocSite) -> Option<crate::ir::ValueId> {
    for (ii, inst) in func.insts.iter().enumerate() {
        if let Inst::Alloc { site: s, .. } = inst
            && *s == site
        {
            let iid = crate::ir::InstId(ii as u32);
            for (vi, vd) in func.values.iter().enumerate() {
                if vd.def == Some(iid) {
                    return Some(crate::ir::ValueId(vi as u32));
                }
            }
        }
    }
    None
}

fn insert_at_block_start(func: &mut crate::ir::Function, block: BlockId, inst: Inst) {
    let iid = crate::ir::InstId(func.insts.len() as u32);
    func.insts.push(inst);
    func.block_mut(block).insts.insert(0, iid);
}

fn insert_after_inst(func: &mut crate::ir::Function, after: u32, inst: Inst) {
    let new_id = crate::ir::InstId(func.insts.len() as u32);
    func.insts.push(inst);
    // Find the block containing InstId(after) and insert after it.
    let after_id = crate::ir::InstId(after);
    for block in &mut func.blocks {
        if let Some(pos) = block.insts.iter().position(|&id| id == after_id) {
            block.insts.insert(pos + 1, new_id);
            return;
        }
    }
}

/// Unused helper kept for Phase 7 alloc rewriting.
#[allow(dead_code)]
fn _placement_ty(_: Placement) -> IrTy {
    IrTy::Ptr
}
