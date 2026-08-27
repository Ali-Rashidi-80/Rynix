# RFC 0001 — Parametric monomorphization (Track G)

## Summary

Accept compile-time monomorphization of `Vec[T]` and `Map[K,V]` for a closed
type set, mapped onto existing mono RT softs ([ADR-0025](../docs/adr/0025-parametric-monomorphization.md)).

## Motivation

Agents and users want `Vec[T]` sugar without waiting for full generics/HM.

## Guide-level explanation

Write `let v: Vec[i64] = …` as today; additional T ∈ {str, bool} via the same
syntax once Phase 36 ships. Legacy `vec_str_new` remains valid.

## Reference-level explanation

See ADR-0025. No ABI break for existing mono softs.

## Drawbacks

Type-set expansion still requires RT + codegen work per instantiation.

## Rationale and alternatives

Full HM / traits deferred (refuse). Pure mono expansion without sugar already
ships; sugar reduces agent friction.

## Prior art

Rust monomorphization; Zig comptime; Rynix ADR-0006 / 0014 / 0016–0018.

## Unresolved questions

None for v1 allow-list.

## Future possibilities

Parametric `Option[T]` after ADR-0024 experience; traits Track R.

---

**Status:** Accepted (Phase 35)
