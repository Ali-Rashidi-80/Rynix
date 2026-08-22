# Deferred: UI frameworks, hot-reload, canvas (v0.1 out of scope)

Date: 2026-08-22  
Status: **Deferred** — not acceptance-gated for v0.1

## Context

Peer projects (e.g. End) market IDE studios, canvas runtimes, and application
frameworks. Rynix Phase 10 focused on **compiler/tooling parity** (LSP, MCP,
arch check, honest benchmarks), not end-user UI stacks.

## Decision

The following remain **explicitly out of scope** until a future milestone with
its own ADR, SPEC, and tests:

- Hot-reload dev servers for GUI apps
- Canvas / game / simulation UI frameworks in `std/`
- Visual “studio” webviews in the editor

## Consequences

- README/COMPARE may note End leads on editor richness; that is honest.
- No ROADMAP ✅ for UI until in-tree harnesses exist.
