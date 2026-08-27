# Phase 31 — Security harden (behavioral)

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md).

Supersedes Phase 27 doc-only gates for MSan/UBSan, cargo-deny, and Windows sandbox.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | CI ASan+UBSan hard; MSan optional doc | `msan_ubsan_rt_enforced` | ✅ |
| B | `deny.toml` + CI `cargo-deny` | `cargo_deny_clean` | ✅ |
| C | `--sandbox=job` Windows Job Object | `windows_sandbox_smoke` | ✅ |
| D | CWE additive `glpat-` | `security_cwe_one_additive` | ✅ |
| E | Contract `phase31_security_harden` | `verify_phase31_security_harden_contract` | ✅ |

Contract: [contracts/phase31_security_harden.contract.toml](contracts/phase31_security_harden.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Security | 9.0 | 9.3 | UBSan CI + cargo-deny + Job Object + `glpat-` |
| C runtime quality | 9.0 | 9.2 | `address,undefined` sanitizer-rt |

## Out of scope

- seccomp-bpf (Track R)
- Claim untrusted `.ryx` is safe
- MSan in same job as ASan (needs instrumented libc++)
