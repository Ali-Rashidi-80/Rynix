//! Source positions, the source map, and string interning for the Rynix
//! compiler.
//!
//! This is the foundation crate: every other compiler crate depends on the
//! types defined here. Design rationale lives in `docs/adr/0003-span-model.md`.
//!
//! - [`Span`] — 8-byte half-open byte range in a *global* offset space.
//! - [`SourceMap`] — owns all loaded files (memory-mapped when possible),
//!   assigns each a contiguous window of the global space, and resolves
//!   offsets to files, lines, and columns on the cold diagnostic path.
//! - [`Interner`] — deduplicating string interner handing out [`Symbol`]s.

mod interner;
mod source_map;
mod span;

pub use interner::{Interner, Symbol};
pub use source_map::{FileId, LineCol, SourceFile, SourceMap};
pub use span::Span;
