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

## Correctness gate

For each workload, every built language must print the **same checksum** as the first
successful language in the run order. CI requires **C ↔ Rynix** match on all 12 rows
(`suite5-check` job, `phase10_gates` test).

## Output

- Console table: challenge × lang × checksum × ms
- JSON: `benchmarks/suite5/suite5_results.json` (`rynix.suite5.v2`)
- With `--summary`: cross-language ms matrix + Rynix/C ratio

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

## Not a crown claim

Numbers are **machine-local**. Suite5 proves **algorithm parity** and records transparent
timings — not that Rynix beats Zig/C on every row.

Compare methodology to End's [suite12](https://github.com/IrMaho/End/tree/main/benchmarks/suite12):
see [benchmarks/README.md](../README.md).

TCP echo RPS: [docs/bakeoff.md](../../docs/bakeoff.md).
