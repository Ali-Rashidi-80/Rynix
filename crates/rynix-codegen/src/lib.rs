//! Textual LLVM IR backend (Phase 7 step 1 — ADR-0005).
//!
//! Emits `.ll` from RIR with whole-program reachability DCE from `@main`.
//! Links against the portable C runtime in `rt/portable.c` via `clang`.

mod emit;
mod reach;

pub use emit::emit_llvm;
pub use reach::{prune_unreachable, reachable_from_main};
