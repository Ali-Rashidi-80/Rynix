# Rynix benchmarks

Honest, reproducible harnesses — **checksum gates** before speed claims.

```text
benchmarks/
├── suite5/          # 12 integer microbenches × 5 languages (see below)
├── suite12/         # checksum-locked End suite12 C ports (MATCH ids only)
scripts/
├── bakeoff_go_echo.go   # optional TCP echo peer
docs/
├── bakeoff.md       # fiber TCP RPS methodology
├── benchmarks.md    # lexer throughput baseline
```

## Quick run

```sh
cargo build -p rynixc
python benchmarks/suite5/run_suite5.py --summary
python benchmarks/suite5/run_suite5.py --langs c,rynix   # CI subset
```

Results: `benchmarks/suite5/suite5_results.json` (machine-local).

---

## Suite5 — 12 workloads × 5 languages

Identical **integer algorithms** in **Rynix · C · Rust · Go · Zig** (one source file per
lang per workload). Each binary prints one checksum line; the harness times wall-clock
of the **finished binary** (compiler time excluded).

| Id | Workload | Category |
|----|----------|----------|
| `alu` | 2M fused integer updates | scalar ALU |
| `nested` | 450² nested loops `% 97` | control + memory |
| `fib` | 5M iterative Fibonacci | loops |
| `hash` | 3M polynomial hash mod 1e9+7 | mix |
| `prime` | trial division to 100k | number theory |
| `sum` | 1.5M sum of squares | reduction |
| `bits` | 25M popcount on LCG stream | bit ops |
| `matrix` | 900k × 4×4 matmul trace | linear algebra |
| `scan` | 8M divisibility scan | branchy loop |
| `powmod` | 2.5M modular exponentiation | math |
| `gcd` | 2.5M Euclidean gcd pairs | number theory |
| `reduce` | 10M ALU reduction | End #12 analogue |

**Gate:** for each workload, **C ↔ Rynix checksum must match** (CI + `phase10_gates`).

Details: [suite5/README.md](suite5/README.md)

---

## vs [End suite12](https://github.com/IrMaho/End/tree/main/benchmarks/suite12)

End ships a **different** 12-challenge suite (SDF raymarch, binary trees, HFT engine,
SHA-256, N-body, …) in unified multi-bench binaries with 5-run statistics.

| | End suite12 | Rynix Suite5 |
|--|-------------|--------------|
| Workloads | 12 heavyweight systems sims | 12 integer microkernels |
| Languages | End, C, Rust, Go, Zig | Rynix, C, Rust, Go, Zig |
| Correctness | checksum per bench | checksum per workload (CI-gated C↔Rynix) |
| Stats | 5 runs + warmup | trimmed median (warmup=3, runs=9; `--summary`) |
| Purpose | product marketing matrix | compiler parity + honest local numbers |

We **do not** claim Suite5 scores are comparable to End suite12 row-by-row — different
algorithms. We **do** match the **12-workload × 5-language** shape with transparent harnesses.

Conceptual mapping (spirit, not algorithm identity):

| End suite12 (theme) | Rynix Suite5 (closest) | Rynix suite12 port |
|---------------------|-------------------------|-------------------|
| Super-Scalar ALU (10M) | `reduce`, `alu` | `alu_reduction.c` (#12) |
| Binary trees | — | `binary_trees.c` (#2) |
| HFT engine | — | `hft_engine.c` (#3) |
| SHA-256 blocks | `hash` | `sha256_blocks.c` (#4) |
| GEMM 512×512 | `matrix` | `gemm_matrix.c` (#10) |
| FSM Lexer stream | lexer gate in `docs/benchmarks.md` | `fsm_lexer.c` (#9) |
| JSON microservice | `json_get_i64` runtime tests (not Suite5) | `json_serializer.c` (#8) |
| DNA / Levenshtein | — | `dna_levenshtein.c` (#7) |
| Monte Carlo BS | — | `monte_carlo_bs.c` (#11) |
| N-body / orbit | — (deferred; End checksum diverges) | skip #5 |
| SDF raymarch | — (deferred; End checksum diverges) | skip #1 |
| TCP / HFT engine | `docs/bakeoff.md` fiber echo RPS | see HFT port |

---

## Other harnesses

| Harness | Doc | Gate |
|---------|-----|------|
| Lexer throughput (~400 MiB/s) | [docs/benchmarks.md](../docs/benchmarks.md) | criterion baseline |
| TCP echo RPS (fibers) | [docs/bakeoff.md](../docs/bakeoff.md) | `tcp_echo_rps.c`, `load_harness.c` |
| Hello binary size | README / `size_echo_gates` | under 300 KiB |

---

## Publishing numbers

1. Run `python benchmarks/suite5/run_suite5.py --summary` on your machine.
2. Attach `suite5_results.json` or paste the summary table.
3. State OS, CPU, clang/rustc/go/zig versions.
4. Never claim victory without checksum OK for every row you cite.
