//! Structured diagnostics for the Rynix compiler.
//!
//! Rynix is AI-native from day one: every diagnostic carries a stable code
//! (`RYX####`, registered in [`codes`] and documented in
//! `docs/diagnostics.md`), precise [`Span`](rynix_span::Span)s, and
//! machine-applicable [`Fix`]es with confidence scores.
//!
//! Renderers:
//! - [`render_json`] — one-line JSON per diagnostic (`rynix.diag.v1`).
//! - [`render_human`] — compact human-readable text (full snippet renderer
//!   lands in Phase 3).
//!
//! Diagnostics may allocate: they are always on the cold path. The hot
//! compiler paths only ever *construct* diagnostics when source is invalid.

mod code;
mod diagnostic;
mod human;
mod json;
mod sink;

pub use code::{CodeInfo, DiagCode, codes};
pub use diagnostic::{Diagnostic, Edit, Fix, Label, Severity, Stage};
pub use human::render_human;
pub use json::render_json;
pub use sink::{CountSink, DiagSink, VecSink};
