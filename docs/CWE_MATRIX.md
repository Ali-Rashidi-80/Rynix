# CWE coverage matrix (honest)

Companion for Phase 27-H (`security_cwe_matrix_or_deferral`).

## Current scanner (`rynixc security` / MCP `rynix_security`)

| CWE | Class | Status | Notes |
|-----|-------|--------|-------|
| **CWE-798** | Use of Hard-coded Credentials | **Implemented** (pattern) | Prefix / assignment heuristics (`sk_live_`, `AKIA`, `password = "`, …). Advisory; not taint. |

This is intentionally a **CWE-798-class** line scanner — not a full audit.

## Additive / related (compiler posture)

| CWE | Theme | Status | Mechanism |
|-----|-------|--------|-----------|
| CWE-78 | OS Command Injection | Partial (emit path) | RIR sanitize + sema reject of `system` / `exec*` / `popen` ([ADR-0023](adr/0023-rir-sanitize.md)) |
| CWE-426 | Untrusted Search Path | Out of scope | Host clang / PATH trust |
| CWE-94 | Code Injection | Deferred | Treating `.ryx` like C; no “safe eval” claim |
| CWE-502 | Deserialization of Untrusted Data | Deferred | No general serde of untrusted blobs in v0.1 |
| CWE-22 | Path Traversal | Deferred | Soft `std::fs` is fopen-backed; no full path sandbox |

## Deferred expansion

Expanding `rynix_security` beyond CWE-798 (e.g. regex XSS, SQLi string
patterns) is **deferred** until there is a SPEC + false-positive budget.
Documented here so Quality-10 does not claim broader CWE theater.
