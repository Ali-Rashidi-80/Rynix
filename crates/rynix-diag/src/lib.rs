//! Structured diagnostics for the Rynix compiler.
//!
//! Rynix is AI-native from day one: every diagnostic carries a stable code
//! (`RYX####`, registered in [`codes`] and documented in
//! `docs/diagnostics.md`), precise [`Span`](rynix_span::Span)s, and
//! machine-applicable [`Fix`]es with confidence scores.
//!
//! Renderers:
//! - [`render_json`] — one-line NDJSON per diagnostic (`rynix.diag.v1`;
//!   schema at `docs/schemas/rynix.diag.v1.json`).
//! - [`render_human`] — annotated source snippets with carets.
//!
//! Diagnostics may allocate: they are always on the cold path. The hot
//! compiler paths only ever *construct* diagnostics when source is invalid.

mod code;
mod diagnostic;
mod human;
mod json;
mod schema;
mod sink;

pub use code::{CodeInfo, DiagCode, codes};
pub use diagnostic::{Diagnostic, Edit, Fix, Label, Severity, Stage};
pub use human::render_human;
pub use json::render_json;
pub use schema::validate_diag_v1;
pub use sink::{CountSink, DiagSink, VecSink};
