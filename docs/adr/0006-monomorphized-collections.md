# ADR-0006: Collections are monomorphized (`Vec[i64]` / `Map[i64, i64]`)

Status: accepted (2026-08-21); **complete for shipping**

## Context

Full parametric polymorphism needs monomorphizing backends, bounds, and
layout specialization. The runtime ships region-backed
`rynix_rt_vec_i64_*` / `map_i64_*`.

## Decision

Shipping collections:

- Types `Vec` / `Map` and applied forms `Vec[i64]` / `Map[i64, i64]` resolve
  to distinct kinds `TypeKind::Vec` / `TypeKind::Map` (both ABI as `ptr`).
- Non-`i64` element/key/value types are type errors.
- Soft builtins `vec_*` / `map_*` plus methods `.push` / `.get` / `.len` /
  `.insert` (dispatched by receiver kind) are the complete API surface.

This **is** the complete collections design — not a temporary stub waiting
for `Vec[T]`.

## Consequences

- Predictable layouts and small binaries.
- Additional monomorphs (e.g. `Vec[str]`) are additive future work, not an
  open hole in the current surface.
