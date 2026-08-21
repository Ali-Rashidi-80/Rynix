//! Rynix Intermediate Representation (RIR): canonical SSA with block arguments.
//!
//! Design notes (Phase 5):
//! - `SoA` layout: dense [`BlockId`] / [`InstId`] / [`ValueId`] handles;
//! - block parameters instead of φ-nodes (Cranelift-style);
//! - every `alloc` carries a stable [`AllocSite`] for later escape analysis;
//! - textual `.rir` for FileCheck-style pass tests.

mod builder;
mod interp;
mod ir;
mod lower;
mod parse;
mod pass;
mod print;
mod verify;

pub use builder::FunctionBuilder;
pub use interp::{interpret_module, InterpError, InterpValue};
pub use ir::*;
pub use lower::lower_module;
pub use parse::{parse_module, ParseError};
pub use pass::{const_fold, dce, run_pipeline, simplify_cfg};
pub use print::print_module;
pub use verify::verify_module;
