# ADR-0003: Global u32 span model with memory-mapped sources

Status: accepted (2026-08-21)

## Context

Every token, AST node, and diagnostic carries source positions. Options:
(a) per-file offsets + FileId in every span (12+ bytes), (b) rustc-style
global u32 offset space where each file occupies a contiguous window
(8 bytes, file recovered by binary search).

## Decision

`Span { lo: u32, hi: u32 }` — 8-byte `Copy`, half-open, addressing a global
offset space managed by `SourceMap`. Files are separated by a 1-byte gap so
spans from different files can never touch. Total source per session is
capped at 4 GiB.

Sources are loaded with `memmap2` (zero-copy) and UTF-8-validated exactly
once at load. Line tables (`line_starts`) are computed once with `memchr` and
used only on the cold diagnostic-rendering path (binary search). Empty files
and files with a BOM fall back to owned strings. We assume source files are
not modified externally during a compilation session (standard mmap
trade-off; documented, not checked).

## Consequences

- `Token` is 12 bytes; AST nodes stay small; side tables index by position.
- 4 GiB total-source cap is acceptable for v0.x and enforced at load time.
- Miri-friendly tests use owned strings; mmap is exercised in integration
  tests only.
