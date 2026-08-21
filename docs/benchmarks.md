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

- Phase 1 initial target: >= 400 MB/s single core on the mixed corpus.
  Status: met within noise on the dev box (358-414 MiB/s interval); the
  authoritative number will come from the quiet Linux CI runner.
- Long-term target: 1 GB/s (SIMD identifier/whitespace scanning, perfect-hash
  keyword lookup, and batched token emission are the known levers, deferred
  until the parser consumes tokens for real).

## History

| Date       | Change                                             | Corpus    |
| ---------- | -------------------------------------------------- | --------- |
| 2026-08-21 | Fix O(n^2) `\r` scan in string literals            | 2.26 MiB/s -> ~640 MiB/s (strings) |
| 2026-08-21 | Skip digit-run validation when literal has no `_`  | numbers +27% |
| 2026-08-21 | Phase 1 close baseline                             | ~388 MiB/s |
