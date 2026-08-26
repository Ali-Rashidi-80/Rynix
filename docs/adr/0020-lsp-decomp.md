# ADR-0020: LSP module decomposition

## Status

Accepted (Phase 26)

## Context

`crates/rynixc/src/lsp_cmd.rs` grew past ~1.6k LOC with protocol framing,
diagnostics, AST walks, resolve helpers, and handlers in one file. That slowed
review and made Phase 26+ LSP work harder without changing behavior.

## Decision

- Split into `crates/rynixc/src/lsp/` with logical modules: `protocol`,
  `diagnostics`, `walk`, `resolve`, `server`, `navigation`, `features`,
  `symbols`, `document`, plus `tests`.
- Keep `lsp_cmd.rs` as a thin `pub use crate::lsp::*` so `main` /
  `lsp_cmd::run` churn stays minimal.
- No LSP behavior or capability changes in this ADR.

## Consequences

- Unit tests stay under `lsp::tests` and must keep passing
  (`document_symbol_lists_fn`, `workspace_symbol_lists_fn`,
  `references_lists_local_uses`, and prior coverage).
- Follow-on LSP features land in the matching submodule, not a monolith.
