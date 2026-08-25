# ADR-0016: `Vec[str]` monomorph (additive)

## Status

Accepted (Phase 23-C)

## Context

[ADR-0014](0014-mono-collections-niche10.md) kept mono `Vec[i64]` / `Map[i64,i64]`
for Niche-10 and deferred further monomorphs. Agents repeatedly need string
vectors without inventing parametric `Vec[T]`.

## Decision

- Add **`TypeKind::VecStr`** displayed as `Vec[str]`.
- Soft API: `vec_str_new` / `vec_str_push` / `vec_str_get` / `vec_str_len`
  (methods `.push` / `.get` / `.len` on `Vec[str]` receivers).
- Runtime: `rynix_rt_vec_str_*` storing `const char *` pointers (NUL C strings;
  no deep copy — caller lifetime / region honesty).
- Do **not** claim parametric `Vec[T]`.

## Consequences

- Gate: `vec_str_roundtrip`, `vec_str_annotation_ok`.
- `Vec[bool]` and other elems remain rejected.
