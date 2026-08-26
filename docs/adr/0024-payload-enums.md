# ADR-0024: Payload enums (`Some(T)` / match bind)

## Status

**Deferred** (Phase 28-D)

## Context

GOLDEN_PATH Phase 28-E would add nullary-payload enums such as `Some(T)` with
match binding once this ADR is **Accepted**. Nullary enum variants and
`Enum::Variant` paths already ship ([ADR-0015](0015-match-enum-variants.md)).
Parametric / payload-carrying enums interact with Track G (generics) design.

## Decision

Defer payload enums and match binds. Do **not** stub `Some` / `None` as theater
without a roundtrip gate. Revisit after Track G ADR acceptance or an explicit
narrow Accepted amendment with gate `enum_payload_match_roundtrip`.

## Consequences

- Phase 28-E is skipped while Status remains Deferred.
- Language surface stays at nullary variants only.
- Gate for this ADR in Phase 28: file exists with Status (this document).
