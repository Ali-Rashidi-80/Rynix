# Phase 37 — Public Track G release (`v0.2.0`)

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md).

**Trigger:** explicit user ask after Phase 36 (this phase). Tag push / GitHub
Release are part of this wave — not auto-started from Phase 36.

Previously documented as HOLD until that ask.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | CHANGELOG cut `[Unreleased]` → `[0.2.0]` (Phases 30–36 / Track G) | `changelog_v020_cut` | ✅ |
| B | This document | file | ✅ |
| C | `git tag v0.2.0` + push on user ask | remote tag | ✅ |
| D | GitHub Release via `release.yml` (`v*` tag) + SHA256SUMS | release job | ✅ |
| E | PRODUCTION_READINESS Track G / `0.2.0` | `production_readiness_v020` | ✅ |
| F | Contract `phase37_release` | `verify_phase37_release_contract` | ✅ |

Contract: [contracts/phase37_release.contract.toml](contracts/phase37_release.contract.toml).

## Axis note

| Axis | Prior (post-30) | After | Gates |
|------|----------------:|------:|-------|
| Security | 9.0 | 9.3 | Phase 31 behavioral (UBSan, cargo-deny, Job Object) |
| C runtime quality | 9.0 | 9.2 | Phase 32 uring TCP + CI sanitizers |
| AI tooling | 9.6 | 9.8 | Track G Vec/Map matrices (Phase 36) |
| Deployment / CI | 9.5 | 9.6 | `v0.2.0` Release + SHA256SUMS |
| Documentation | 9.7 | 9.8 | PHASE37 + GOLDEN_REMAINING close |

## Version policy

- `v0.1.0` = Niche-10 (do not re-tag)
- `v0.1.1` = Quality-10 public cut (Phases 21–29)
- `v0.2.0` = Track G shipped (Phases 30–36) — this release

## Out of scope

- Absolute-10 / Niche-11 claims
- Phase 38 agent surface (optional follow-on)
- HKT / unbounded parametric traits
