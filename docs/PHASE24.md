# Phase 24 — Map[str,i64] + product example

After Phase 23. Thin collection mono + callable product surface.

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| A | `Map[str, i64]` mono ([ADR-0017](adr/0017-map-str-i64-mono.md)) | `map_str_i64_roundtrip` |
| B | `examples/12_http_vec_map_str.ryx` (path_param HTTP + Vec[str] + Map[str,i64]) | `example_http_vec_map_str_checks` |

Contract: [contracts/phase24_map_str.contract.toml](contracts/phase24_map_str.contract.toml).

## Out of scope

- `Map[str, str]` / parametric maps
- Payload enum match
- Push/release
