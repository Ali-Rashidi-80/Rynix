//! Arena-allocated abstract syntax tree for Rynix.
//!
//! Design (ADR-0004):
//! - every node lives in a [`AstArena`] (bumpalo newtype);
//! - identifiers are interned [`Symbol`]s, never owned `String`s;
//! - no `Box` / `Rc` / `Drop` types inside nodes;
//! - [`NodeId`] is a dense `u32` handle for `SoA` side tables in later phases.
//!
//! Lifetimes: a parsed tree is valid for as long as the arena that owns it.

mod arena;
mod node;
mod print;

pub use arena::{AstArena, NodeId};
pub use node::*;
pub use print::dump_module;
