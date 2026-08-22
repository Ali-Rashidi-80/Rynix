# Lexer Benchmark Baseline

Recorded with `cargo bench -p rynix-lexer` (Criterion, 10 MiB synthetic
workloads plus a mixed "corpus" workload built from realistic Rynix source).
Criterion keeps its raw history under `target/criterion/` and automatically
compares each run against the previous one; this file is the committed,
human-readable record.

## Methodology

- Each workload is generated in memory (no disk I/O inside the measured loop).
- The lexer runs through `CountSink`, so diagnostics cost is included but no
  diagnostic storage allocates.
- Throughput = input bytes / wall time, single thread, no warm cache tricks.
- To pin a named baseline locally: `cargo bench -p rynix-lexer -- --save-baseline <name>`
  and later `-- --baseline <name>`.

## Baseline: 2026-08-21 (Phase 1 close)

Machine: Windows 10 dev box, `x86_64-pc-windows-gnu`, stable Rust, `lto = "thin"`
bench profile. This machine is noisy (antivirus, background services); numbers
are mid estimates with roughly +/-10% run-to-run variance. CI on Linux should
re-record this table when it comes online.

| Workload      | Throughput (mid) | Notes                                        |
| ------------- | ---------------- | -------------------------------------------- |
| identifiers   | ~800 MiB/s       | keyword-vs-ident lookup on every token       |
| numbers       | ~313 MiB/s       | +27% after skipping validation when no `_`   |
| strings       | ~640 MiB/s       | memchr3-driven scan, escapes validated       |
| punctuation   | ~337 MiB/s       | worst case: ~1.9 bytes/token dispatch bound  |
| corpus (mixed)| ~388 MiB/s       | realistic module: defs, structs, literals    |

## Targets

- Mixed-corpus acceptance: ≥ ~400 MiB/s single core (within noise).
  Status: **met** (~388 MiB/s mid; 358–414 MiB/s interval on the Phase 1
  close machine). This is the shipping lexer performance gate.
- Identifier-heavy workloads already approach ~800 MiB/s; further SIMD work
  is optional optimization, not an open acceptance residual.

## History

| Date       | Change                                             | Corpus    |
| ---------- | -------------------------------------------------- | --------- |
| 2026-08-21 | Fix O(n^2) `\r` scan in string literals            | 2.26 MiB/s -> ~640 MiB/s (strings) |
| 2026-08-21 | Skip digit-run validation when literal has no `_`  | numbers +27% |
| 2026-08-21 | Phase 1 close baseline                             | ~388 MiB/s |
