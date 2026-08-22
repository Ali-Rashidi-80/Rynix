# ADR-0009: Agent contracts as toolchain evidence (not a second language)

Date: 2026-08-22  
Status: **Accepted** (design) — implementation tracked under Phase 11

## Context

[End](https://github.com/IrMaho/End) ships an in-language Agent Contract System
(`feature` / `skill` / `task` / `evidence` / `verify`) aimed at AI pair programming.

Rynix already has **machine-readable** agent surfaces:

- `rynix.diag.v1` NDJSON diagnostics
- MCP tools (graph, impact, slice, eval, patch, explain_alloc, …)
- `Architecture.toml` + `rynixc arch check`
- JSON schemas under `docs/schemas/`

Duplicating End’s surface syntax would be **copy-paste competition** without
improving Rynix’s strength (test-gated honesty).

## Decision

1. **Do not** add End-style `feature`/`skill`/`task` keywords to `.ryx` for v0.1.
2. Express agent contracts as **toolchain artifacts**:
   - Architecture boundaries stay in `Architecture.toml`
   - Task/evidence lives in repo markdown or CI (existing CONTRIBUTING / ROADMAP gates)
   - Optional later: `rynixc verify --contract=PATH.toml` that checks named tests
     exist and pass (evidence = cargo/test names), without new language syntax
3. If a language-level contract DSL is ever needed, it requires:
   - SPEC section + ADR superseding this one
   - In-tree parser/sema tests
   - One worked example under `examples/` with CI gate

## Consequences

- README / END_PEER_GAP list agent contracts as a **product gap**, not a silent ✅.
- Phase 11 may add `rynixc verify` (toml contracts → test evidence) without
  widening the language surface.
- Keeps Rynix differentiated: **auditable toolchain** over **syntax spectacle**.

## Alternatives considered

| Option | Rejected because |
|--------|------------------|
| Clone End keywords into `.ryx` | Copy-paste; no unique value |
| Soft-parse contracts in comments | Unreliable for agents |
| Defer forever | Blocks honest peer comparison narrative |

## References

- [END_PEER_GAP.md](../END_PEER_GAP.md)
- [ADR-0007](0007-deferred-ui-frameworks.md) (similar “don’t ship End frameworks”)
- End docs: `AGENT_CONTRACT_SYSTEM.md` (peer; not mirrored)
