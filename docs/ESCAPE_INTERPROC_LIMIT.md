# Escape analysis — interprocedural limit (Phase 29-D)

**Gate:** `escape_interproc_or_limit_doc`.

## What ships

`rynix_rir` escape analysis is **intraprocedural points-to + interprocedural SCC**
placement (`crates/rynix-rir/src/escape/analyze.rs`). Agents see results via
`rynixc check --explain-alloc` / MCP `rynix_explain_alloc`.

## Limit

No new measured interprocedural win is claimed in Phase 29. Cross-function
precision beyond the current SCC merge remains a Track R / future wave item.
This document is the honesty gate for 29-D.

## Pointers

- SPEC memory model (escape / region)
- ADR history under `docs/adr/` for memory decisions
