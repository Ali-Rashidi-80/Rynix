# ADR-0025: Parametric monomorphization (`Vec[T]` / `Map[K,V]`)

## Status

**Accepted** (Phase 35)

## Context

Q-Core used monomorphized soft collections (`Vec[i64]`, `Vec[str]`, `Vec[bool]`,
fixed `Map` variants). Track G introduces **compile-time monomorphization** of
`Vec[T]` / `Map[K,V]` for a closed type set — not HKT, traits, or HM inference.

## Decision

1. Syntax: `Vec[T]`, `Map[K, V]` where T/K/V are concrete types from an allow-list.
2. Lowering: each instantiation reuses the existing mono soft/RT symbols
   (`vec_*`, `vec_str_*`, `vec_bool_*`, `map_*`, …).
3. Migration: legacy soft names remain aliases (Phase 36-D).
4. Refuse v1: HKT, traits/vtable, unbounded type params, `Option[T]` parametric
   (payload enums stay ADR-0024 narrow).

## Consequences

- Gate spike: `vec_t_i64_compat_spike` (Phase 35).
- Ship matrices: Phase 36 contract `phase36_track_g`.
- RFC process: [rfcs/README.md](../../rfcs/README.md).
