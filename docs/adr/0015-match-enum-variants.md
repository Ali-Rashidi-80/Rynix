# ADR-0015: `match` on nullary enum variant paths

## Status

Accepted (Phase 21-C)

## Context

Phase 17-C shipped nullary enum values as i64 discriminants with `==` compare.
`match` patterns were limited to int/bool/`_` ([SPEC](../SPEC.md) §3), so agents
rewrote enum control flow as `if` chains. Extending `match` to bare variant
idents is a thin surface win; `Vec[str]` remains deferred under
[ADR-0014](0014-mono-collections-niche10.md).

## Decision

Allow a match arm pattern that is a single-segment path resolving to a
**nullary** enum variant (same resolution as a value expression). Parser
disambiguation: an `Ident` starts a pattern only when the next significant
token is `Newline` / `end` / `else` / `Eof` — so `print_i64(...)` and `x = 1`
remain arm-body statements.

Payload variants (`Some(T)`) stay out of scope until a later ADR.
Qualified **`Enum::Variant`** paths (and match arms) are accepted in Phase 23-B.

## Consequences

- SPEC `match_pat` gains Ident (nullary variant).
- Sema/RIR already typecheck and lower `MatchPat::Literal` Path → discriminant.
- Gate: `enum_match_variant_roundtrip` (check + run prints `2`).
