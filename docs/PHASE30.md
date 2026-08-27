# Phase 30 — Public Quality-10 release (`v0.1.1`)

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md).

**Trigger:** explicit user ask (this phase). Tag push / GitHub Release are part of
this wave — not auto-started from Phase 28/29.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | CHANGELOG cut `[Unreleased]` → `[0.1.1]` (Phases 21–29); fix compare URLs | `changelog_v011_cut` | ✅ |
| B | This document | file | ✅ |
| C | `git tag v0.1.1` + push on user ask | remote tag | ✅ |
| D | GitHub Release via `release.yml` (`v*` tag) + SHA256SUMS | release job | ✅ |
| E | GPG optional documented | docs row | ✅ |
| F | PRODUCTION_READINESS scoreboard (11 axes) | `production_readiness_scoreboard` | ✅ |
| G | Contract `phase30_release` | `verify_phase30_release_contract` | ✅ |

Contract: [contracts/phase30_release.contract.toml](contracts/phase30_release.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Deployment / CI | 8.6 | 9.5 | `v0.1.1` Release + SHA256SUMS workflow |
| Documentation | 9.6 | 9.7 | PRODUCTION_READINESS Quality-10 scoreboard |

## Version policy

- `v0.1.0` = Niche-10 (do not re-tag)
- `v0.1.1` = Quality-10 public cut (Phases 21–29)
- `v0.2.0` = Track G — Phase 37, explicit ask only

## Out of scope

- Track G in release notes as shipped
- Absolute-10 claims
- Auto-start of Phase 31+
