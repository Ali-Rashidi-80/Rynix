# Adding End as a 6th Suite5 language

Goal: run **the same 12 integer algorithms** as C/Rust/Go/Zig/Rynix, not End’s
suite12 unified binary (different programs).

## Status

| Piece | Status |
|-------|--------|
| `build_end()` in `run_suite5.py` | ✅ wired (skips if toolchain missing) |
| 12× `{challenge}.end` sources | ✅ in-tree (ported from Suite5 C/Rynix) |
| Status | ✅ sources + harness; **local checksum parity vs C/Rynix on all 12** (needs `endc` on PATH) |

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

Each `.end` file embeds `suite5_print_i64` (same `SUITE5_BENCH` contract as `bench.h`).

## Port notes

- Syntax follows End suite12 style (`pub fn`, `val`/`mut`, `for i in N`, `@import_c`).
- Sources are **best-effort** until validated with a local `end build`.
- If End’s CLI flags differ (`--strip` vs `--release`), adjust `build_end()` in `run_suite5.py`.

## suite12 (End’s own matrix)

End’s `benchmarks/suite12/suite12_*.` files are **not** Suite5-compatible.
Do **not** compare suite12 ms to Suite5 ms in README tables.
