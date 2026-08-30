# ADR-0026: Parametric `Option[T]` (allow-list mono)

## Status

**Accepted** (Phase 40)

## Context

ADR-0024 ships narrow payload enums via user-defined `enum Opt / Some(i64) / None`.
Track G (ADR-0025) established allow-list monomorphization for `Vec[T]` / `Map[K,V]`.
Agents and tutorials benefit from standard `Option[i64]` / `Option[str]` syntax without
requiring a user enum per call site.

## Decision

1. Syntax: `Option[T]` where `T` is **`i64` or `str` only** (closed allow-list).
2. Lowering: each instantiation is a **builtin payload enum** using the same RT layout
   as ADR-0024 (`rynix_rt_enum_payload_*`).
3. `None` / `Some(x)` resolve in **Option context** (annotation, match scrutinee, or
   payload type of `Some` argument) — not duplicated in module scope.
4. Refuse v1: `Option[bool]`, unbounded `T`, traits, `&T`, reuse of user enum names
   for builtin Option.
5. Gates: `option_t_i64_match_roundtrip`, `option_t_str_match_roundtrip`.

## Consequences

- Amend ADR-0024: user payload enums remain; ADR-0026 adds builtin sugar.
- Amend ADR-0025 refuse line: parametric `Option[T]` allow-list is **in** Track G pattern.
- Phase 41 ships implementation + roundtrips.

## Amendments

- ADR-0024: unchanged behavior for user enums; builtin Option is additive.
- ADR-0025: line 19 `Option[T]` parametric → **allow-list only via ADR-0026**.
