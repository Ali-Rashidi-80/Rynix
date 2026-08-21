//! Source positions, the source map, and string interning for the Rynix
//! compiler.
//!
//! This is the foundation crate: every other compiler crate depends on the
//! types defined here. Design rationale lives in `docs/adr/0003-span-model.md`.

pub struct Placeholder;
