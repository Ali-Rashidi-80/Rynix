//! Minimal LSP server on stdio: full-sync documents, diagnostics, go-to-definition,
//! hover, completion, rename, references, and workspace symbols.

#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms,
    clippy::needless_borrow,
    clippy::single_match,
    clippy::too_many_lines,
    clippy::unnecessary_filter_map,
    clippy::unnecessary_wraps
)]

mod diagnostics;
mod document;
mod features;
mod navigation;
mod protocol;
mod resolve;
mod server;
mod symbols;
mod walk;

#[cfg(test)]
mod tests;

pub use server::run;
#[allow(unused_imports)] // re-exported for `lsp_cmd` / callers
pub use server::LanguageServer;

pub(crate) use document::Document;

#[cfg(test)]
pub(crate) use protocol::LspRequest;

#[cfg(test)]
pub(crate) use resolve::{
    completion_items, def_index_at, find_definition_span, find_workspace_fn_def, hover_at,
    name_at_offset, reference_spans,
};
