# Phase 35 — Track G design lock

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md).

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | RFC 0001 + ADR-0025 Accepted | ADR/RFC files | ✅ |
| B | `Vec[i64]` via type path still equals mono IR | `vec_t_i64_compat_spike` | ✅ |
| C | No mono deletion until Phase 36-D | review | ✅ |

Contract: [contracts/phase35_track_g_adr.contract.toml](contracts/phase35_track_g_adr.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Architecture | 9.5 | 9.6 | ADR-0025 Accepted |

## Out of scope

- Traits/vtable; HM full; deleting mono softs
