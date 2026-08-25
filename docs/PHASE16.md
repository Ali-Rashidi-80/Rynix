# Phase 16 — Honesty deepen + path_param HTTP + MCP path-first

**Status:** **Phase 16 Waves 0–D complete** (2026-08-25)  
**After:** Phase 15 complete ([PHASE15.md](PHASE15.md)) · Niche-10 map ([adr/0013-niche-10-scorecard.md](adr/0013-niche-10-scorecard.md))

## North star

1. Refresh peer honesty (Suite5 + End) without theater.
2. Fix stale production-readiness / Sigstore-lite wording.
3. Ship **one** product HTTP deepen: numeric path segment → JSON i64.
4. Make MCP `rynix_graph` **path-first** (read disk; fail-closed).

Phase 16 is the **base** of the Niche-10 path (Phases 16→20). Completing it does
**not** certify Niche-10 — that is Phase 20-D.

## Order

`0 (docs + ADRs) → A (Suite5/peer) → B (docs honesty) → C (path_param) → D (MCP path)`

## Locked decisions

| ID | Lock |
|----|------|
| P16-L0 | Niche-10 is the 10/10 target ([ADR-0013](adr/0013-niche-10-scorecard.md)); Absolute-10 vs Go refused |
| P16-L1 | Waves `0→A→B→C→D` in order |
| P16-L2 | Raft / consensus product deferred ([ADR-0012](adr/0012-deferred-consensus.md)) |
| P16-L3 | llama.cpp embed refused until real FFI + smoke ADRs |
| P16-L4 | Path param = prefix + decimal digits only (e.g. `/items/` + `42`) |
| P16-L5 | MCP path-first starts with `rynix_graph`; other tools follow in Phase 19 |
| P16-L6 | ROADMAP ✅ only with named in-tree gates |
| P16-L7 | No push unless explicitly requested |

## Gates

| Wave | Gate | Theme |
|------|------|--------|
| 0 | docs + ADR-0012/0013 present | locks |
| A | Suite5 summary artifact + peer tables | honesty benches |
| B | PRODUCTION_READINESS / ROADMAP wording | docs honesty |
| C | `http_loop_path_param` | path_param HTTP |
| D | `mcp_graph_path_file` | MCP reads disk path |

## Complete

Waves 0–D shipped. Next: Phase 17 language surface (see ROADMAP).

## Refuse

Raft Stable rows, llama embed theater, full WASI, keep-alive/framework in this
phase (those are Phase 18), Absolute-10 marketing claims.
