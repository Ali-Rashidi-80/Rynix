# Phase 36 — Track G ship (minimum viable)

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md). ADR-0025.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | `Vec[T]` T∈{i64,str,bool} | `vec_t_roundtrip_matrix` | ✅ |
| B | `Map[K,V]` matrices | `map_kv_roundtrip_matrix` | ✅ |
| C | `std/collections.ryx` facade | `std_collections_facade_ok` | ✅ |
| D | Legacy soft aliases | `legacy_mono_alias_ok` | ✅ |
| E | Contract | `verify_phase36_track_g_contract` | ✅ |

Contract: [contracts/phase36_track_g.contract.toml](contracts/phase36_track_g.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| AI tooling | 9.7 | 9.8 | Vec/Map matrices + facade |

## Out of scope

- HKT; traits; unbounded params
