# Escape analysis — interprocedural SCC (Phase 32)

**Gate:** `escape_interproc_improvement_gate`.

## What ships

`rynix_rir` escape analysis: intraprocedural points-to + **interprocedural SCC
fixpoint** (`crates/rynix-rir/src/escape/analyze.rs`).

Phase 32 adds named regression `interproc_scc_mutual_recursion_arg_escape` proving
mutual-recursion modules propagate `ArgEscape` across the SCC (not stuck at
`NoEscape`).

## Remaining limit

Full context-sensitive / heap-shape analysis remains Track R. Agents see results
via `rynixc check --explain-alloc` / MCP `rynix_explain_alloc`.
