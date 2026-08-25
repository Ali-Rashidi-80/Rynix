# ADR-0014: Mono `Vec[i64]` / `Map[i64,i64]` retained for Niche-10

Date: 2026-08-25  
Status: **Accepted** — after Phase 17-A/B/C language gates

## Context

[ADR-0006](0006-monomorphized-collections.md) shipped monomorphized collections.
Niche-10 ([ADR-0013](0013-niche-10-scorecard.md)) required struct `str` fields,
index assign, and nullary enum values — not generic `Vec[T]`.

## Decision

- Keep **mono** `Vec[i64]` / `Map[i64, i64]` as the shipping collections surface.
- Language Niche-10 for collections is satisfied by **honesty** (this ADR) plus
  Phase 17-A/B/C gates (`struct_str_field_roundtrip`, `index_assign_ok`,
  `enum_value_roundtrip`).
- Additional monomorphs (`Vec[str]`, …) remain additive future work, not a
  Niche-10 blocker.

## Consequences

- Docs must not claim parametric collections.
- Agents must not invent `Vec[T]` stubs to chase Absolute-10 theater.
