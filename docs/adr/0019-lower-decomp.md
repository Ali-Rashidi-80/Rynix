# ADR-0019: Decompose `lower.rs` into `lower/`

## Status

Accepted (Phase 26)

## Context

`crates/rynix-rir/src/lower.rs` grew to ~5k LOC (Suite5 closed forms, loop-carried
SSA, soft calls). [GOLDEN_PATH.md](../GOLDEN_PATH.md) Phase **26-A** requires a
behavior-identical split targeting ~900 LOC/file for maintainability -- not a
language or IR change.

## Decision

- Replace `lower.rs` with `lower/mod.rs` plus sibling sources:
  `types`, `host_math`, `recognizers`, `loop_carried`, `ctx`, `value`,
  `loop_kernels`, `stmt`, `expr`.
- Keep a **single module namespace** via `include!` from `mod.rs` so visibility
  and call graphs stay identical (no logic edits; `pub use lower::lower_module`
  in `lib.rs` unchanged).
- Do **not** change lowering semantics, peepholes, or Suite5 host folds.

## Consequences

- Gate: existing `rynix-rir` / `rynixc` lowering coverage (e.g.
  `map_str_str_roundtrip`) must stay green.
- Follow-on: **26-B** `lsp_cmd` -> `lsp/` (ADR-0020 when written).
