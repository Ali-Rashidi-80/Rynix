# Deferred: UI frameworks, hot-reload, canvas (v0.1 out of scope)

Date: 2026-08-22  
Reaffirmed: 2026-08-23  
Status: **Deferred** — not acceptance-gated for v0.1

## Context

Peer projects (e.g. End) market IDE studios, canvas runtimes, and application
frameworks. Rynix Phase 10–11 focused on **compiler/tooling parity** (LSP, MCP,
arch check, honest benchmarks, real HTTP/TLS/WS/IOCP), not end-user UI stacks.

WebSocket **protocol** framing (`std/ws.ryx`, `rt/src/ws.c`) is **not** a UI
framework — it is networking. Canvas/game studios remain out of scope here.

## Decision

The following remain **explicitly out of scope** until a future milestone with
its own ADR, SPEC, and tests:

- Hot-reload dev servers for GUI apps
- Canvas / game / simulation UI frameworks in `std/`
- Visual “studio” webviews in the editor

**Do not** ship stub canvas APIs or empty `std/ui.ryx` wrappers to match End
marketing rows.

Revisit only when:

1. A dedicated UI ADR names the first surface (e.g. immediate-mode debug HUD)
2. In-tree harnesses exist (render smoke or headless golden)
3. Agent/editor integration is specified without conflicting with ADR-0009

## Consequences

- README/COMPARE may note End leads on editor richness; that is honest
- No ROADMAP ✅ for UI until in-tree harnesses exist
- END_PEER_GAP “game/canvas” stays open; WebSocket networking may be ✅ separately
- Competitive “beyond Surpass” wave (2026-08-23): **closed by this ADR** — shipping
  a stub studio would violate AGENTS.md; revisit criteria above are unchanged
