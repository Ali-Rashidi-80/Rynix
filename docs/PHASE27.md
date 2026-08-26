# Phase 27 — Security posture (Quality-10)

Parent: [GOLDEN_PATH.md](GOLDEN_PATH.md). Raises Security axis toward ≥9.0.
Does **not** claim “untrusted `.ryx` is safe.”

## Waves

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | `--sandbox=docker\|none` for clang link (opt-in; default `none`) | `sandbox_docker_smoke` *or* [SANDBOX_SKIP_MATRIX.md](SANDBOX_SKIP_MATRIX.md) | [0022](adr/0022-build-sandbox.md) |
| B | RIR sanitize: reject `system`/`exec*`/`popen`/`dlopen` escapes | `sanitize_rejects_exec` | [0023](adr/0023-rir-sanitize.md) |
| C | MSan+UBSan enforce on `rt/` smokes (Linux CI) | `msan_ubsan_rt_clean` | — ([SANITIZER_SCAFFOLD.md](SANITIZER_SCAFFOLD.md)) |
| D | Fuzz targets: parse + seeds | `fuzz_new_targets_seeded` | — |
| E | Threat model (STRIDE) | file + link from [SECURITY.md](../SECURITY.md) | — ([SECURITY_THREAT_MODEL.md](SECURITY_THREAT_MODEL.md)) |
| F ∥ | Windows Job Object sandbox *or* deferral | `windows_sandbox_or_deferral` | 0022 amend ([WINDOWS_SANDBOX_DEFERRAL.md](WINDOWS_SANDBOX_DEFERRAL.md)) |
| G ∥ | `emit-ll` / no-clang-link fast path smoke | `emit_ll_no_link_smoke` | — |
| H ∥ | CWE matrix beyond 798 | `security_cwe_matrix_or_deferral` | — ([CWE_MATRIX.md](CWE_MATRIX.md)) |

Contract: [contracts/phase27_security.contract.toml](contracts/phase27_security.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Security | 7.6 | ≥9.0 (target) | sandbox + sanitize + threat model + CWE honesty |

## Out of scope

- seccomp as hard Quality-10 requirement
- Coq fiber proof
- Claiming untrusted `.ryx` is safe to compile/run
