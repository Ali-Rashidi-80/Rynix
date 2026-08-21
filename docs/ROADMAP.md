# Rynix Development Roadmap

Rynix is an AI-native systems/backend language: canonical syntax, structured
diagnostics, Zero-GC memory (escape → stack/region/heap), colorless fibers,
and an LLVM backend targeting small binaries.

Phases are acceptance-gated. Irreversible decisions live in [docs/adr/](adr/).

## Conventions

- Binary: `rynixc` · sources: `.ryx` · IR text: `.rir` · codes: `RYX####`
- One atomic step ≈ one commit · English docs · zero-config `fmt`
- Windows (gnu) first-class through codegen; Linux for io_uring + ASan CI

## Pipeline

```
.ryx → lexer → parser/AST → sema → RIR (SSA)
    → escape + regions + free → LLVM (.ll) + ThinLTO → binary + rynix-rt
```

## Honesty legend

- ✅ acceptance met with in-tree tests
- ◐ real code; residual limits listed (not silent)

---

## Phase 0 — Workspace ✅

Cargo workspace, pedantic lints, `unsafe_op_in_unsafe_fn = deny`, ADRs 0001–0005.

## Phase 1 — Lexer ✅

Zero-alloc cursor, `RYX0001..0006`, mmap, proptest tiling, zero-alloc proof,
criterion (~388 MiB/s mixed; 400 MiB/s target within noise; 1 GB/s stretch open).

## Phase 2 — Parser / AST ✅

Arena AST, Pratt, recovery, dumps; formatter in Phase 9.

## Phase 3 — Diagnostics ✅

Human + `rynix.diag.v1` JSON schema; MCP tools completed Phase 9.

## Phase 4 — Sema ✅

Scopes, types, `#^ error`, soft std + `tensor`/`signal`/`agent`.

## Phase 5 — RIR ✅

SoA SSA, block args, DCE/const-fold/simplify-cfg/BCE, interpreter.
**Indexing** with `bounds_check` / `load_index`.
**Immutable lets** bind as SSA values; **`let mut`** stays alloc/load/store
(Braun sealing on blocks). **`for` / `break` / `continue`** lower to real CFG.
**Field** access via `field_offsets` + `gep_i64`.
`match` still reserved (not implemented).

## Phase 6 — Escape ✅

Lattice, region inject, `Free { site, ptr }` → `rynix_rt_heap_free`.
`#^ alloc:` + `#^ free-at` (unit) covered.

## Phase 7 — LLVM ✅ / ◐

Textual `.ll`, reachability DCE, heap free codegen, hello size gate `<300KiB`
(when clang present). Inkwell (ADR-0005 step 2) still future.

## Phase 8 — Runtime ✅ / ◐

- Fibers (Win32 / ucontext), colorless read/write/sleep
- **TCP:** non-blocking listen/accept/connect/recv/send (fiber-safe)
- **Echo + RPS:** `rt/tests/tcp_echo_rps.c` (local loopback RPS floor)
- **io_uring:** Linux `RYNIX_RT_URING` syscall ring for read/write; init on
  `rynix_rt_run`; portable fallback if setup fails
- Pipe echo + fiber smoke + ASan CI on Ubuntu

Residual: not a published RPS bake-off vs Go/Tokio; uring accept not yet SQE’d
(TCP accept remains non-blocking poll + yield).

## Phase 9 — Std / tooling / AI ✅ / ◐

- CLI: build/run/test/fmt/mcp-serve · `rynix.toml`
- MCP: diagnostics, format, explain_alloc, compile, ast_query, apply_fix
- **Vec/Map:** region-backed `rynix_rt_vec_i64_*` / `map_i64_*` + soft builtins
  (i64 monomorphized — language generics not in the type system yet)
- Soft net/time/io surface · Presburger-lite BCE · smart primitives

---

## Milestones

| Id | Criterion | Status |
|----|-----------|--------|
| M0–M6 | scaffold → escape | ✅ |
| M7 | native LLVM + size gate | ✅ (clang) |
| M8 | fiber TCP echo + RPS smoke; uring path on Linux | ✅ / ◐ (see Phase 8) |
| M9 | std surface + MCP + fmt + BCE + Vec/Map runtime | ✅ / ◐ (no language generics) |

## Explicit residual (not silently closed)

1. Language-level generics (Vec/Map\<T\>) — runtime is i64-mono today
2. io_uring accept/connect SQEs + load-test vs Go/Tokio
3. `match` expression; full method calls
4. inkwell / in-process LLVM; lexer 1 GB/s stretch
5. Differential LLVM binary vs interpreter (interp corpus is in-tree)

## CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml): `cargo test` on
Ubuntu + Windows; ASan builds of fiber/echo/TCP smokes; uring-flag compile on Linux.
