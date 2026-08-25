# Phase 13 — WASM emit + manifest optimize

**Status:** **Phase 13 complete** (waves A–C green, 2026-08-25)  
**After:** Phase 12 complete ([LEAD_AHEAD.md](LEAD_AHEAD.md)) · peer verdict ([VERDICT.md](VERDICT.md))

## North star

1. `rynixc emit-ll file.ryx --target=wasm32-unknown-unknown` writes `.ll` with a
   wasm32 triple that **clang `--target=wasm32-unknown-unknown -c`** accepts.
2. `[build].optimize` in `rynix.toml` actually controls RIR optimize for `build`/`run`
   (closes Phase 12 L5).
3. Dual-path bounded HTTP JSON routing on the existing fiber/RT path (not a
   framework).
4. No WASI runtime port, no browser games, no End WASM toy theater.

## Order

`0 (docs locks) → A (wasm emit-ll) → B (manifest optimize)` — A before B only for
independence; may ship same day with separate commits when possible.

## Locked decisions

| ID | Lock |
|----|------|
| P13-L1 | Extend **`emit-ll --target=`**, not a fake `emit-wasm` until a `.wasm` file is produced by a gated path. |
| P13-L2 | Allowed target v1: **`wasm32-unknown-unknown` only** (reject unknown `--target=`). |
| P13-L3 | Smoke gate: emit-ll + `clang --target=wasm32-unknown-unknown -c` on arith-only fixture; **skip** if clang lacks wasm. |
| P13-L4 | No `rt/` link for wasm v1 (no fibers/TCP/WASI). |
| P13-L5 | `build` optimize: CLI `--opt` / `--no-opt` when present wins; else `[build].optimize`; else **`true`** (preserve prior default). |
| P13-L6 | Textual LLVM + external clang only ([ADR-0005](adr/0005-textual-llvm-ir-first.md)); no C11 ([ADR-0008](adr/0008-deferred-c11-backend.md)). |

## Gates

| Wave | Gate test | Theme |
|------|-----------|--------|
| A | `emit_ll_wasm32_clang_accepts` | wasm32 `.ll` + clang `-c` |
| B | `build_respects_manifest_optimize` | `[build].optimize = false` skips RIR opt path (observable) **or** `--no-opt` / manifest round-trip |
| C | `http_loop_2paths` | dual-path bounded JSON GET routing |

## Refuse

Full WASI libc, `build --target=wasm32` linking host `rt/`, CDN, UI, Raft/GGUF stubs.
