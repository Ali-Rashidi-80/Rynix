//! Escape analysis and region inference (Phase 6 — Zero-GC core).
//!
//! Lattice: `NoEscape < ArgEscape < RegionEscape < GlobalEscape`.
//! Placement: `NoEscape` → stack; `ArgEscape`/`RegionEscape` → bump region;
//! `GlobalEscape` → heap.

#![allow(clippy::module_name_repetitions)]

mod analyze;
mod explain;
mod lattice;
mod regions;

pub use analyze::{analyze_escape, module_call_graph, EscapeReport, SiteInfo};
pub use explain::{explain_alloc_human, explain_alloc_json};
pub use lattice::{Escape, Placement};
pub use regions::inject_regions;
