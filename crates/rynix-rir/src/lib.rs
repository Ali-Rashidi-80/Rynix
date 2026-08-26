//! Rynix Intermediate Representation (RIR): canonical SSA with block arguments.
//!
//! Design notes (Phase 5):
//! - `SoA` layout: dense [`BlockId`] / [`InstId`] / [`ValueId`] handles;
//! - block parameters instead of φ-nodes (Cranelift-style);
//! - every `alloc` carries a stable [`AllocSite`] for later escape analysis;
//! - textual `.rir` for FileCheck-style pass tests.
//!
//! Phase 6 adds escape analysis (`escape`) over allocation sites.

mod builder;
pub mod escape;
mod bounds;
mod interp;
mod ir;
mod lower;
mod parse;
mod pass;
mod print;
mod sanitize;
mod verify;

pub use bounds::eliminate_bounds_checks;
pub use builder::FunctionBuilder;
pub use escape::{
    analyze_escape, explain_alloc_human, explain_alloc_json, inject_regions, module_call_graph,
    Escape, EscapeReport, Placement, SiteInfo,
};
pub use interp::{interpret_module, interpret_module_print, InterpError, InterpValue};
pub use ir::*;
pub use lower::lower_module;
pub use parse::{parse_module, ParseError};
pub use pass::{const_fold, dce, run_pipeline, simplify_cfg};
pub use print::print_module;
pub use sanitize::{is_dangerous_ext_name, sanitize_module};
pub use verify::verify_module;
