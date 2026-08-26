# ADR-0017: `Map[str, i64]` monomorph (additive)

## Status

Accepted (Phase 24)

## Context

[ADR-0016](0016-vec-str-mono.md) shipped `Vec[str]`. Agents need string-keyed
maps (counts, path scores) without parametric `Map[K,V]`.

## Decision

- Add **`TypeKind::MapStrI64`** displayed as `Map[str, i64]`.
- Soft API: `map_str_i64_new` / `_insert` / `_get` / `_len` (methods `.insert` /
  `.get` / `.len` on `Map[str, i64]` receivers).
- Runtime: open-addressed table of `const char *` keys (NUL C strings; pointer
  identity + `strcmp`; no key deep-copy).
- Do **not** claim parametric maps; further monos additive ([ADR-0018](0018-map-str-str-mono.md)).

## Consequences

- Gates: `map_str_i64_roundtrip`, `example_http_vec_map_str_checks`.
