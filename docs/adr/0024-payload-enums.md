# ADR-0024: Payload enums (`Some(i64)` / `Some(str)` / match bind)

## Status

**Accepted** (amended Phase 33) — narrow payloads **without** parametric `Option[T]`.

## Context

GOLDEN_PATH Phase 28 deferred payload enums pending Track G. Quality-10 Remaining
Path Phase 33 ships a **narrow** surface: i64 and str payloads only, decoupled
from parametric generics (Track G / ADR-0025).

## Decision

1. Enum variants may carry a single payload of type `i64` or `str`.
2. Match arms may bind `Variant(name)` for payload variants; nullary arms unchanged.
3. Representation: payload-carrying enums lower as a **pointer** to `{disc: i64, payload}`
   (i64 payload inline; str as pointer). Nullary-only enums remain `i64` discriminants
   ([ADR-0015](0015-match-enum-variants.md)).
4. Do **not** add parametric `Option[T]` or traits in this ADR.
5. Gates: `enum_payload_i64_match_roundtrip`, `enum_payload_str_match_roundtrip`.

## Consequences

- Phase 33 implements narrow payloads before Track G.
- SPEC + skill updated in the same band.
- Revisit parametric enums only after ADR-0025.
