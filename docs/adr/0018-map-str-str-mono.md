# ADR-0018: `Map[str, str]` monomorph (additive)

## Status

Accepted (Phase 25)

## Context

[ADR-0017](0017-map-str-i64-mono.md) shipped `Map[str, i64]`. Agents need
string-to-string maps (HTTP headers-shaped tables, labels) without parametric
`Map[K,V]`. [GOLDEN_PATH.md](../GOLDEN_PATH.md) reserves **0018** for this mono
only (R16); generics remain Track G.

## Decision

- Add **`TypeKind::MapStrStr`** displayed as `Map[str, str]`.
- Soft API: `map_str_str_new` / `_insert` / `_get` / `_len` (methods `.insert` /
  `.get` / `.len` on `Map[str, str]` receivers).
- Runtime: open-addressed table of `const char *` keys and `const char *` values
  (NUL C strings; pointer identity + `strcmp` on keys; no deep-copy of key/val).
- Do **not** claim parametric maps.

## Consequences

- Gates: `map_str_str_roundtrip`, `example_map_str_str_product_checks`.
- Next monos remain additive; parametric → new ADR after 0024.
