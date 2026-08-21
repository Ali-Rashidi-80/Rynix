# Rynix Development Roadmap

Rynix is a systems/backend programming language designed to be AI-native
first: canonical syntax (one way to do anything), machine-consumable
diagnostics, deterministic Zero-GC memory management, colorless concurrency
on io_uring, and an LLVM backend producing sub-1MB binaries.

This roadmap is atomic and phase-gated: no phase closes without tests and an
explicit acceptance criterion. Irreversible decisions are recorded as ADRs in
[docs/adr/](adr/).

## Naming and conventions

- Compiler binary: `rynixc`. Source extension: `.ryx`. Textual IR: `.rir`.
- Diagnostic codes: `RYX####` (see [diagnostics.md](diagnostics.md)).
- One atomic step = one commit. English docs; canonical formatting only.
- Development platform: Windows (gnu toolchain) is fully supported through
  codegen; the runtime phase (io_uring) develops/tests on WSL2 or Docker and
  CI runs on Linux.

## Pipeline overview

```
.ryx (mmap) -> Lexer (zero-alloc tokens) -> Parser (arena AST)
    -> Sema (names + types) -> RIR (canonical SSA)
    -> Escape Analysis + Region Inference + injected free
    -> LLVM IR + ThinLTO + whole-program DCE -> binary (<1MB) + rynix-rt
Every stage emits structured diagnostics (JSON / MCP).
```

## Honesty legend

- ✅ = acceptance criteria met with tests in-tree.
- ◐ = real implementation with known gaps (listed).
- ○ = not started / stub only.

## Phase 0 — Workspace scaffold ✅

- Cargo workspace, stable toolchain pin, pedantic lints,
  `unsafe_op_in_unsafe_fn = deny`, release profile `lto=fat`,
  `codegen-units=1`, `panic=abort`.
- Crates: `rynix-span`, `rynix-diag`, `rynix-lexer`, `rynix-ast`,
  `rynix-parser`, `rynixc` (+ later crates).
- Acceptance: green `cargo build`/`cargo test`, docs in place.

## Phase 1 — Zero-allocation lexer ✅

- Zero-alloc cursor, structured `RYX0001..0006`, mmap sources.
- Tests: unit, insta, proptest tiling, zero-alloc counter, fuzz, criterion
  (mixed corpus ~388 MiB/s — within noise of 400 MiB/s target; 1 GB/s stretch).

## Phase 2 — Parser and arena AST ✅

- Arena AST, Pratt parser, error recovery, s-expression dumps.
- Canonical formatter (`rynixc fmt`) landed in Phase 9.

## Phase 3 — JSON diagnostics ✅

- Dual human/JSON renderers, `rynix.diag.v1` schema + golden tests.
- Full MCP server tools completed in Phase 9 / backlog.

## Phase 4 — Semantic analysis ✅

- Scopes, types, `#^ error RYX2xxx` directives.
- Soft builtins: `print`, `sleep_ms`, `yield`, `now_ms`, `fiber_run`,
  `tensor` (compile-time length check), `signal`, `agent`.

## Phase 5 — RIR ✅ (with noted deferrals)

- SoA SSA, block args, DCE/const-fold/simplify-cfg, interpreter oracle.
- **Indexing:** `bounds_check` + `load_index` + array layout `[len|elems…]`.
- **BCE:** interval / Presburger-lite pass (`eliminate_bounds_checks`) removes
  proven-safe checks (const index/len and recovered array lengths).
- Still deferred: full Braun on-the-fly SSA for all mutables; `match`; field
  lowering; full `for`/`break`/`continue` CFG.

## Phase 6 — Escape analysis ✅

- Escape lattice, region inject, `Free { site, ptr }`, `--explain-alloc`.
- Gaps: `#^ free-at` corpus incomplete; no sanitizer CI yet.

## Phase 7 — LLVM backend ◐

- Textual `.ll`, reachability DCE, `emit-ll`/`build`, heap free codegen.
- **Size gate:** `crates/rynixc/tests/size_echo_gates.rs` asserts hello
  `< 300KiB` when clang is available.
- Gaps: inkwell step 2; differential LLVM vs interpreter not wired to every
  corpus; http-echo binary size gate deferred until TCP lands.

## Phase 8 — Runtime fibers ◐

- Win32 Fibers / ucontext, portable colorless read/write/sleep, spawn/yield/run.
- **Echo smoke:** `rt/tests/echo_smoke.c` + gated test (pipe round-trip).
- **`--runtime=uring`:** still stub hooks (`rt/src/uring_stub.c`) — full SQE
  park + liburing is Linux follow-up. M8 “target RPS echo server” is **not**
  claimed done.

## Phase 9 — Stdlib, tooling, AI ✅ / ◐

- CLI: `build` / `run` / `test` / `fmt` / `mcp-serve`.
- MCP tools: `diagnostics`/`rynix_check`, `rynix_format`,
  `rynix_explain_alloc`, `compile`, `ast_query`, `apply_fix`.
- Canonical formatter (zero config).
- Soft std prelude + `std/` docs (`core`/`io`/`fs`/`net`/`time`/`json`);
  executable collections are language arrays + builtins (no generic Vec yet).
- Smart primitives: `tensor(len, […])` shape check; `signal`/`agent` soft.
- Presburger-lite BCE: landed (const + recovered lengths).

## Open follow-ups (explicit, not silently closed)

1. Real Linux io_uring backend (SQE + fiber park) and TCP listen/accept.
2. Load-tested fiber HTTP echo + RPS vs Go/Tokio; echo `<1MB` size gate.
3. Generic `Vec`/`Map` in std on region allocators.
4. Full Braun SSA for mutables; field/method/`for` CFG completeness.
5. Sanitizer CI, `#^ free-at` corpus, differential codegen oracle expansion.
6. Lexer 1 GB/s stretch; inkwell migration (ADR-0005 step 2).

## Milestones

- M0–M6: met.
- M7: native binaries via LLVM — met with size gate when clang present.
- M8: fiber portable echo smoke met; **io_uring RPS echo not met**.
- M9: std surface + MCP + fmt + BCE + smart-primitive experiments met.
