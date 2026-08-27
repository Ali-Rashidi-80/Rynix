# Golden Remaining Path (post Q-Core 25–29)

**Status: CLOSED** — Phases **30–38** shipped (`v0.1.1`, `v0.2.0`, LSP codeAction).  
**Parent SoT:** [GOLDEN_PATH.md](GOLDEN_PATH.md) (Q-Core locked).  
**Refuse:** R1–R19 unchanged from GOLDEN_PATH §2.  
**Follow-on:** Track R / Refuse only — not a new golden wave.

## Two-layer honesty

| Layer | Meaning |
|-------|---------|
| Q-Core 25–29 | Closed with honesty / deferral gates (doc-backed where noted) |
| Remaining 30–38 | Behavioral implementation supersedes doc-only gates — **done** |

## Version policy

| Tag | Meaning |
|-----|---------|
| `v0.1.0` | Niche-10 (Phases 16–20) — do not re-tag |
| `v0.1.1` | Public Quality-10 band (Phases 21–29) — Phase 30 (**explicit ask**) |
| `v0.2.0` | Track G shipped — Phase 37 (**explicit ask** after 36) |

Release cuts stay **explicit ask** only (never auto push/tag from a prior phase).

## Phases

| Phase | Theme | Status |
|------:|-------|--------|
| 30 | Public release `v0.1.1` | ✅ [PHASE30.md](PHASE30.md) |
| 31 | Security harden (UBSan, cargo-deny, Job Object, CWE) | ✅ [PHASE31.md](PHASE31.md) |
| 32 | Runtime / HTTP (uring TCP, Bearer, escape) | ✅ [PHASE32.md](PHASE32.md) |
| 33 | Language (payloads, struct bool, multiline, Vec[bool]) | ✅ [PHASE33.md](PHASE33.md) |
| 34 | Track C depth (tutorials, CONTRIBUTING, RFC, E-14/E-15) | ✅ [PHASE34.md](PHASE34.md) |
| 35 | Track G ADR-0025 + compat spike | ✅ [PHASE35.md](PHASE35.md) |
| 36 | Track G ship (Vec[T]/Map[K,V] matrices) | ✅ [PHASE36.md](PHASE36.md) |
| 37 | Public `v0.2.0` | ✅ [PHASE37.md](PHASE37.md) |
| 38 | Agent surface remainder (LSP codeAction) | ✅ [PHASE38.md](PHASE38.md) |

## Gate supersession

| Old (Q-Core doc) | New (behavioral) | Phase |
|------------------|------------------|------:|
| `msan_ubsan_rt_clean` | `msan_ubsan_rt_enforced` | 31 |
| `cargo_deny_or_deferral` | `cargo_deny_clean` | 31 |
| `windows_sandbox_or_deferral` | `windows_sandbox_smoke` | 31 |
| `uring_recv_send_completion_smoke` | `uring_tcp_recv_send_completion_smoke` | 32 |
| `http_auth_or_method_gate` | `http_bearer_header_soft_gate` | 32 |
| `escape_interproc_or_limit_doc` | `escape_interproc_improvement_gate` | 32 |

## Quality-10 “complete” (Remaining) — met

1. Phases 30–36 contracts verify green  
2. [PRODUCTION_READINESS.md](../PRODUCTION_READINESS.md) scoreboard: axes ≥9.0, Security ≥9.0 with behavioral gates  
3. No doc-only gate for ROADMAP “Implemented” rows  
4. Niche-10 certification unchanged ([NICHE10.md](NICHE10.md))  
5. Refuse R1–R19 unviolated  
6. Phase 37 `v0.2.0` + Phase 38 codeAction shipped  

**Not required (still Track R / Refuse):** playground, 1 GiB/s, Niche-11, Absolute-10.

## Execution order (historical)

Wave 0 → Phase 30 (explicit ask) → 31 → 32 → 33 → 34 → 35 → 36 →
Phase 37 (explicit ask) → Phase 38 — **complete**.
