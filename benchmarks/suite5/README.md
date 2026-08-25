# Suite5 — 12-workload cross-language microbenchmarks

Identical integer algorithms in **Rynix / C / Rust / Go / Zig / End** (Zig and End
optional if toolchain missing).
Each program prints one checksum; the harness times the finished binary.

Default timing: **warmup=3**, **runs=9**, reported ms = **trimmed median** (drops min/max when runs≥5).
Override: `SUITE5_WARMUP`, `SUITE5_RUNS`, or `--warmup` / `--runs`.

```sh
# from repo root
cargo build -p rynixc
python benchmarks/suite5/run_suite5.py
python benchmarks/suite5/run_suite5.py --summary          # markdown matrix + Rynix/C ratio
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix
python benchmarks/suite5/run_suite5.py --langs c,rynix,end   # End peer when endc installed
python benchmarks/suite5/run_suite5.py --langs c,rynix    # CI gate subset
```

End ports: see [END_INTEGRATION.md](END_INTEGRATION.md). Honest End gap analysis: [docs/END_PEER_GAP.md](../../docs/END_PEER_GAP.md).
## Workloads (12)

| Id | Description |
|----|-------------|
| `alu` | 2M fused integer updates |
| `nested` | 450² nested loops + `% 97` |
| `fib` | 5M iterative Fibonacci steps |
| `hash` | 3M polynomial hash mod `1e9+7` |
| `prime` | trial division count to 100k |
| `sum` | 1.5M sum of squares |
| `bits` | 25M popcount on LCG stream |
| `matrix` | 900k × 4×4 matmul trace |
| `scan` | 8M divisibility scan (mod 3 / 7) |
| `powmod` | 2.5M modular exponentiation |
| `gcd` | 2.5M Euclidean gcd pairs |
| `reduce` | 10M ALU reduction (End suite12 #12 analogue) |

Trip counts pass through an **opaque** barrier (`opaque_i64` / `suite5_opaque_i64`) so
Suite5 sources cannot collapse to a single host-evaluated constant from a literal `n`.
Checksums must still match across languages.

Compilers may **strength-reduce** recognized patterns (closed forms, `ctpop`, matrix fib,
hash polynomials, …). That is a real optimization when disclosed; Suite5 measures
**binary wall-clock for the same checksum**, not identical instruction mixes.
Literal-bound host-fold outside Suite5 remains a compiler feature (unit-tested separately).

## Correctness gate

For each workload, every built language must print the **same checksum** as the first
successful language in the run order. CI requires **C ↔ Rynix** match on all 12 rows
(`suite5-check` job, `phase10_gates` test).

## Honesty

- Opaque barriers block literal trip-count folding of Suite5 sources.
- Strength reduction is allowed and should be named in result Notes / docs.
- Do not claim Suite5 proves identical work across languages after reductions.
- Phase 12 product waves (HTTP loop, structs, `std`, LSP) are **usefulness** gates —
  not substitutes for this harness ([LEAD_AHEAD.md §8](../../docs/LEAD_AHEAD.md)).

## Output

- Console table: challenge × lang × checksum × ms
- JSON: `benchmarks/suite5/suite5_results.json` (`rynix.suite5.v2`)
- With `--summary`: cross-language ms matrix + Rynix/C ratio

### Latest head-to-head vs End (2026-08-25, Windows; warmup=3, runs=9)

Checksums OK on all 12 for C, Rynix, and End. Artifact:
`suite5_summary_2026-08-25_vs_end.txt`. Peer regressions + port notes:
[END_INTEGRATION.md](END_INTEGRATION.md).

| Challenge | c | rynix | end | Winner |
|-----------|--:|------:|----:|--------|
| alu | 11.4 | 5.8 | 7.9 | rynix |
| nested | 6.9 | 5.3 | 5.3 | tie |
| fib | 9.6 | 6.6 | 8.3 | rynix |
| hash | 19.7 | 5.5 | 16.5 | rynix |
| prime | 11.7 | 8.3 | 64.0 | rynix |
| sum | 13.1 | 6.6 | 5.3 | end |
| bits | 457.6 | 92.2 | 418.9 | rynix |
| matrix | 11.2 | 5.5 | 6.5 | rynix |
| scan | 17.2 | 5.8 | 13.6 | rynix |
| powmod | 16.8 | 5.4 | 16.8 | rynix |
| gcd | 178.9 | 113.4 | 243.2 | rynix |
| reduce | 27.5 | 5.3 | 15.6 | rynix |

**Rynix 10 · End 1 · tie 1.** Strength reduction disclosed for large Rynix/C gaps.
Authoritative narrative: [docs/VERDICT.md](../../docs/VERDICT.md).

### Prior multi-lang snapshot (2026-08-25, no End)

| Challenge | c | rust | go | zig | rynix | Rynix/C |
|-----------|--:|-----:|---:|----:|------:|--------:|
| alu | 8.7 | 8.2 | 12.2 | 9.7 | 5.5 | 0.64 |
| nested | 7.2 | 7.4 | 13.0 | 6.2 | 6.4 | 0.88 |
| fib | 7.7 | 6.8 | 9.4 | 7.6 | 5.2 | 0.67 |
| hash | 18.1 | 17.9 | 18.2 | 18.6 | 5.7 | 0.31 |
| prime | 10.9 | 9.8 | 15.0 | 10.6 | 8.1 | 0.74 |
| sum | 6.0 | 5.8 | 9.6 | 6.2 | 5.9 | 0.98 |
| bits | 457.1 | 438.1 | 648.8 | 477.8 | 88.6 | 0.19 |
| matrix | 6.9 | 8.2 | 68.8 | 7.6 | 5.4 | 0.77 |
| scan | 16.6 | 15.4 | 14.8 | 17.5 | 6.3 | 0.38 |
| powmod | 15.6 | 15.0 | 16.5 | 15.7 | 5.6 | 0.36 |
| gcd | 168.9 | 170.9 | 220.0 | 167.5 | 119.8 | 0.71 |
| reduce | 13.4 | 14.9 | 20.8 | 16.9 | 6.9 | 0.52 |

Rynix fastest on **11/12** here (`nested`: Zig edged ahead). Big ratios on
`bits`/`hash`/… reflect disclosed strength reduction, not “same asm as C.”

### One-shot PGO workflow

```sh
python benchmarks/suite5/run_pgo_suite.py          # train + c,rynix baseline + pgo + analyze
python benchmarks/suite5/run_pgo_suite.py --full   # baseline all 5 langs
python benchmarks/suite5/run_pgo_suite.py --skip-train   # reuse profiles
```

Sample PGO delta (Windows, 2026-08-22; **re-run locally** — often ±5%, sometimes regresses):

| Workload | baseline → pgo | delta |
|----------|----------------:|------:|
| nested | 7.8 → 6.9 ms | **−11%** |
| sum | 8.2 → 6.9 ms | **−16%** |
| alu | 8.7 → 8.1 ms | −7% |
| matrix | 7.0 → 7.8 ms | +11% (regression) |
| powmod | 15.4 → 16.1 ms | +5% (regression) |

PGO is **optional** and not a merge gate; default builds skip `--pgo-use`.

## Toolchain notes

| Lang | Build flags (harness) |
|------|------------------------|
| C | `clang -O3` |
| Rust | `rustc -O -C lto=thin` |
| Go | `CGO_ENABLED=0`, `-ldflags=-s -w` |
| Zig | `zig build-exe -O ReleaseFast -lc` |
| End | `end build … --strip` (skipped if missing) |
| Rynix | `rynixc build … --runtime=portable` |

### Optional LLVM PGO (Rynix only)

Train a profile from one representative run per workload, then rebuild with `--pgo-use`:

```sh
python benchmarks/suite5/pgo_train.py
export RYNIX_PGO_PROFDATA=target/suite5/pgo   # optional: directory of per-workload profiles
python benchmarks/suite5/run_suite5.py --langs rynix \
  --pgo-use target/suite5/pgo \
  --json-out benchmarks/suite5/suite5_results_pgo.json
python benchmarks/suite5/analyze_results.py   # includes PGO delta section
```

Each workload gets its own `target/suite5/pgo/<name>.profdata` (do not merge profiles across binaries).
Training runs use `SUITE5_BENCH=1` (same hot path as timed harness) with 3 merged runs by default
(`SUITE5_PGO_TRAIN_RUNS`). PGO deltas are machine-local; several workloads may show no gain or regress.

GitHub Actions: run workflow **Suite5 PGO** (`workflow_dispatch`) for CI-style PGO numbers.

Requires `llvm-profdata` on PATH (LLVM install). Results vary by machine; not used in merge gates.

Zig 0.16+: sources use `@divTrunc` / `@rem` and libc `printf` for stdout checksums.

## Limits

Numbers are **machine-local**. Suite5 proves **checksum parity** and records transparent
timings — not that Rynix executes the same instruction mix as Zig/C on every row.

Compare methodology to End's [suite12](https://github.com/IrMaho/End/tree/main/benchmarks/suite12):
see [benchmarks/README.md](../README.md).

TCP echo RPS: [docs/bakeoff.md](../../docs/bakeoff.md).
