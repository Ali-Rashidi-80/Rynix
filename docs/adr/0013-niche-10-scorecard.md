# ADR-0013: Niche-10 scorecard (Phases 16→20)

Date: 2026-08-25  
Status: **Accepted** — defines the mandatory 10/10 target

## Context

“Must reach 10/10” is ambiguous. Absolute parity with Go/Rust/nginx across every
axis is unrealistic in a short horizon and invites theater. Rynix’s product claim
is **systems language + agent toolchain + offline-first packages**.

## Decision

**Niche-10** is the acceptance target. Absolute-10 vs Go is refused.

An axis may be scored **10** only when its gate row below is green in-tree.
Certification happens in Phase 20-D (`docs/NICHE10.md`), not at Phase 16 close.

| Axis | Gate condition (summary) |
|------|--------------------------|
| Compiler UX | One-path INSTALL Win/Linux; `new→build→run`; NDJSON |
| Runtime I/O | portable + IOCP Win smoke + uring TCP Linux smoke |
| HTTP | path_param (P16) + header + bounded body + bounded keep-alive (P18) |
| TLS/WS/crypto | TLS on product serve/client path; WS + SHA/HMAC documented |
| Packages | local+sparse+lock+attest UX; offline-first forever unless network ADR |
| WASM | host-import `print_i64` + Node smoke (not full WASI) |
| MCP | path-first graph/impact/patch; fail-closed |
| LSP | completion + rename + existing diag/hover/def |
| Benches | Suite5 artifact fresh + CI C↔Rynix honesty |
| Docs | PRODUCTION_READINESS matches tree; no Sigstore-false claims |
| Language | struct i64+str; index assign; enum values; mono Vec/Map via ADR-0014 |

**Still out of Niche-10:** llama embed, Raft product, UI/wgpu, CDN-required registry,
nginx RPS parity claims.

## Consequences

- Phases 16–20 implement the map; Phase 16 alone is not Niche-10.
- Agents must not mark axes 10 without linked gates.
- Follow-on Absolute-10 work needs a new ADR, not silent scope creep.
