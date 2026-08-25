# Phase 15 — Run emitted `.wasm` (still no WASI)

**Status:** **Phase 15 Wave A complete** (2026-08-25)  
**After:** Phase 14 complete ([PHASE14.md](PHASE14.md)) · peer verdict ([VERDICT.md](VERDICT.md))

## North star

1. Closing the Phase 14 loop: `emit-wasm` does not only write `\0asm` bytes —
   a host can **instantiate** the module and call `main` successfully.
2. Gate uses **Node** `WebAssembly` (widely available); skip cleanly if `node`
   is missing. Optional `wasmtime` is out of Wave A.
3. Still **no** WASI libc, no host `rt/` link, no browser UI, no llama.cpp /
   Raft theater (those stay Phase 15+ candidates only with real FFI gates).

## Locked choice (among Phase 15 candidates)

| Candidate | Decision |
|-----------|----------|
| Execute freestanding `.wasm` from `emit-wasm` (Node instantiate) | **Lock Wave A** |
| Skills-as-docs agent pack | Later (docs-only; ADR-0009) |
| llama.cpp / GGUF FFI | Later — only with real FFI + smoke ([LEAD_AHEAD](LEAD_AHEAD.md) §0b) |
| Raft client | Refuse until Jepsen-class tests exist |

## Order

`0 (docs locks) → A (Node runs emit-wasm main)`

## Locked decisions

| ID | Lock |
|----|------|
| P15-L1 | Wave A proves **execution**, not a second magic-byte check. Fixture: `testdata/wasm_arith.ryx` → `main() == 42`. |
| P15-L2 | Runner v1: **Node** `WebAssembly.compile` / `instantiate` + call export `main`. Skip if `node` missing. |
| P15-L3 | No WASI imports required for the fixture (empty import object). |
| P15-L4 | Do not claim browser games, Component Model, or WASI preview1. |
| P15-L5 | Textual LLVM + clang link path unchanged ([PHASE14.md](PHASE14.md)); this wave only adds a run gate. |

## Gates

| Wave | Gate test | Theme |
|------|-----------|--------|
| A | `emit_wasm_node_runs_main` | Node instantiates `.wasm`; `main()===42` |

## Refuse

Full WASI, `rt/` in wasm, UI/canvas, GGUF reimplementation, Raft Stable rows,
Sigstore Rekor theater, Suite5 micro-opt theater.
