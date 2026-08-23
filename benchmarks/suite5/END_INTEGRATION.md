# Adding End as a 6th Suite5 language

Goal: run **the same 12 integer algorithms** as C/Rust/Go/Zig/Rynix, not End’s
suite12 unified binary (different programs).

## Status

| Piece | Status |
|-------|--------|
| `build_end()` in `run_suite5.py` | wired (skips if toolchain missing; honors `ENDC_PATH`) |
| 12× `{challenge}.end` sources | in-tree ports of Suite5 C/Rynix algorithms |
| Fairness | opaque trip counts via `opaque_i64` + `getenv("SUITE5_OPAQUE")` |
| Correctness | local checksum parity vs C/Rynix on all 12 (needs `endc` on PATH) |
| CI | `.github/workflows/ci.yml` job `suite5-with-end` (skip if no endc) |

## What these `.end` files are (honesty)

- They are **Rynix-maintained Suite5 ports**, not copies of End’s official
  [suite12](https://github.com/IrMaho/End/tree/main/benchmarks/suite12).
- End’s suite12 uses **different** workloads (SDF, HFT, SHA-256, …). Do **not**
  compare suite12 ms to Suite5 ms.
- Algorithm bodies match the C/Rynix Suite5 sources (same checksums). Compilers
  may still strength-reduce patterns; that is disclosed in README Notes.

## Fairness contract (must match other langs)

| Requirement | End port |
|-------------|---------|
| Opaque trip / scale inputs | `val n = opaque_i64(…)` then `for i in n` (matrix: opaque `per`) |
| Opaque barrier | `getenv("SUITE5_OPAQUE")` side effect inside `opaque_i64` |
| Timing sink | `suite5_print_i64` honors `SUITE5_BENCH` like `bench.h` |

**Audit note (2026-08-23):** an earlier revision defined `opaque_i64` but still used
literal `for i in N`, so End could const-fold while peers could not. That is fixed
in-tree; re-run Suite5 after pulling.

## Safety

`endc` emits a sibling `.c` next to the input `.end`. The harness copies each
source into `target/suite5/` before building so it **never overwrites**
`benchmarks/suite5/{name}.c`.

## Prerequisites

1. Build [End](https://github.com/IrMaho/End) and put `end` / `endc` on PATH.
2. From Rynix repo root:

```sh
python benchmarks/suite5/run_suite5.py --langs c,rynix,end --summary
# or full matrix:
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary
```

## Port notes

- Syntax follows End suite12 style (`pub fn`, `val`/`mut`, `for i in N`, `@import_c`).
- If End’s CLI flags differ (`--strip` vs `--release`), adjust `build_end()` in `run_suite5.py`.
- `bits.end` uses a software popcount loop (End `>>` quirks); Rynix may lower the
  same source shape to `@llvm.ctpop` — a disclosed compiler win, not a harness cheat.
