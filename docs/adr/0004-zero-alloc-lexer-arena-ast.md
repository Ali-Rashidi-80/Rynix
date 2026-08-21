# ADR-0004: Zero-allocation lazy lexer, arena AST, minimal dependencies

Status: accepted (2026-08-21)

## Context

The front-end performance targets (microsecond-scale lexing, GB/s
throughput) rule out per-token heap traffic. Designs considered: eager
token vector (SoA buffer, Carbon-style), flat index-based AST (Zig-style),
lazy iterator lexer + typed arena AST (rustc-style).

## Decision

1. The lexer is a lazy `Cursor` producing `Copy` tokens on demand — zero
   heap allocations on the error-free path (verified by a `GlobalAlloc`
   counter test). Tokens are spans; token text is always a slice of the
   mmap'd source. Diagnostics may allocate (cold path only).
2. The AST (Phase 2) lives in a bump arena (`bumpalo` behind an `AstArena`
   newtype). Nodes are `&'arena` references; child lists are `&'arena [T]`;
   no `Box`/`Rc`/`String`/`Drop` types inside nodes. Strings are interned
   `Symbol(u32)`.
3. Dependency policy: every external crate must be justified here or in a
   follow-up ADR. Approved: `memmap2` (source loading), `memchr` (SIMD
   scanning), `rustc-hash` (interner map), `bumpalo` (AST arena),
   `serde`/`serde_json` (diagnostics JSON only, never on hot paths).
   Dev-only: `insta`, `proptest`, `criterion`, `libfuzzer-sys`.

A flat SoA AST remains a measured future optimization; the typed-arena
design is proven at rustc scale and keeps Phase 2 ergonomic.

## Consequences

- Parser lookahead uses a small fixed ring buffer, not a token vector.
- The zero-allocation guarantee is enforced by a test, not by convention.
