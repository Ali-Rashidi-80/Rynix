# Golden Remaining Path (post Q-Core 25–29)

**Parent SoT:** [GOLDEN_PATH.md](GOLDEN_PATH.md) (Q-Core locked).  
**Refuse:** R1–R19 unchanged from GOLDEN_PATH §2.  
**North star:** Quality-10 engineering maturity (axes ≥9.0, Security ≥9.0) without
Absolute-10, Niche-11, or Track G theater.

## Two-layer honesty

| Layer | Meaning |
|-------|---------|
| Q-Core 25–29 | Closed with honesty / deferral gates (doc-backed where noted) |
| Remaining 30–37 | Behavioral implementation supersedes doc-only gates |

## Version policy

| Tag | Meaning |
|-----|---------|
| `v0.1.0` | Niche-10 (Phases 16–20) — do not re-tag |
| `v0.1.1` | Public Quality-10 band (Phases 21–29) — Phase 30 |
| `v0.2.0` | Track G shipped — Phase 37 (explicit ask only) |

## Phases

| Phase | Theme | Trigger |
|------:|-------|---------|
| 30 | Public release `v0.1.1` | Explicit user ask |
| 31 | Security harden (UBSan, cargo-deny, Job Object, CWE) | After 30 |
| 32 | Runtime / HTTP (uring TCP, Bearer, escape) | After 31 |
| 33 | Language (payloads, struct bool, multiline, Vec[bool]) | After 32 |
| 34 | Track C depth (tutorials, CONTRIBUTING, RFC, E-14/E-15) | After 33 |
| 35 | Track G ADR-0025 + compat spike | After 34 |
| 36 | Track G ship (Vec[T]/Map[K,V] matrices) | After 35 |
| 37 | Public `v0.2.0` | Explicit ask after 36 |
| 38 | Agent surface remainder (optional) | After 33 |

## Gate supersession

| Old (Q-Core doc) | New (behavioral) | Phase |
|------------------|------------------|------:|
| `msan_ubsan_rt_clean` | `msan_ubsan_rt_enforced` | 31 |
| `cargo_deny_or_deferral` | `cargo_deny_clean` | 31 |
| `windows_sandbox_or_deferral` | `windows_sandbox_smoke` | 31 |
| `uring_recv_send_completion_smoke` | `uring_tcp_recv_send_completion_smoke` | 32 |
| `http_auth_or_method_gate` | `http_bearer_header_soft_gate` | 32 |
| `escape_interproc_or_limit_doc` | `escape_interproc_improvement_gate` | 32 |

## Quality-10 “complete” (Remaining)

1. Phases 30–36 contracts verify green  
2. [PRODUCTION_READINESS.md](../PRODUCTION_READINESS.md) scoreboard: axes ≥9.0, Security ≥9.0 with behavioral gates  
3. No doc-only gate for ROADMAP “Implemented” rows  
4. Niche-10 certification unchanged ([NICHE10.md](NICHE10.md))  
5. Refuse R1–R19 unviolated  

**Not required:** Track R full, playground, 1 GiB/s, Niche-11.

## Execution order

Wave 0 (this file + GOLDEN_PATH/ROADMAP hygiene) → Phase 30 (explicit) →
31 → 32 → 33 → 34 → 35 → 36 → Phase 37 (explicit) → Phase 38 optional.
