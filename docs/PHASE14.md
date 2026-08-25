# Phase 14 — Real `.wasm` via clang (no WASI)

**Status:** **Phase 14 complete** (Waves A–C, 2026-08-25)  
**After:** Phase 13 complete ([PHASE13.md](PHASE13.md)) · peer verdict ([VERDICT.md](VERDICT.md))

## North star

1. `rynixc emit-wasm file.ryx -o out.wasm` writes a **real WebAssembly binary**
   (magic `\0asm`) by emitting wasm32 LLVM IR then linking with
   `clang --target=wasm32-unknown-unknown` (no host `rt/`, no WASI libc).
2. Gate skips cleanly when the host clang cannot target wasm32.
3. Sigstore-lite as a **protocol** is refused; Wave B ships an honest local
   digest bundle instead (`rynix.attest.v1`). Wave C adds triple-path HTTP loop
   smokes — still not a general router.

## Locked choice (among Phase 14 candidates)

| Candidate | Decision |
|-----------|----------|
| `emit-ll` → real `.wasm` via clang (no full WASI) | **Lock Wave A** (closes [P13-L1](PHASE13.md)) |
| Local package lock/sign (Sigstore-lite) | **Wave B as local digest attest** (not Rekor) |
| Deeper I/O (HTTP ≥3 paths / fiber smokes) | **Follow-on Wave C** (`http_loop_3paths`) |

## Order

`0 (docs locks) → A (emit-wasm + clang link smoke) → B (local digest attest) → C (HTTP 3-path loop)`

## Locked decisions

| ID | Lock |
|----|------|
| P14-L1 | Ship **`rynixc emit-wasm`** only because it produces a real `.wasm` file (not a renamed `.ll`). |
| P14-L2 | Link flags v1: `--target=wasm32-unknown-unknown -nostdlib -Wl,--no-entry -Wl,--export-all` (+ `-Wno-override-module`). No WASI sysroot. |
| P14-L3 | No `rt/` / soft-runtime link for wasm v1 (same honesty as P13-L4). Arith / pure IR only for the smoke fixture. |
| P14-L4 | Smoke gate: `emit-wasm` → file exists, size > 0, bytes start with `\0asm`; **skip** if clang lacks wasm link. |
| P14-L5 | Textual LLVM + external clang only ([ADR-0005](adr/0005-textual-llvm-ir-first.md)); no in-process LLVM JIT for wasm. |
| P14-L6 | Suite5 `nested`/`sum` residue vs End is noise-level; **no** Phase 14 wave for micro-opts unless a ≤1h compiler win with checksum lock. |
| P14-L7 | Wave B is **local digest attest** (`rynix.attest.v1.json`), not Rekor/Fulcio/OIDC. Name the file after the schema; do not claim Sigstore protocol. |
| P14-L8 | `deps --attest` writes lock + attest; `--attest-verify` fails on missing file or `lock_sha256`/pin mismatch. Gate: `deps_attest_write_verify_and_tamper`. |
| P14-L9 | Wave C adds **`http_serve_loop_3paths_json_i64`** (bounded triple-route GET); not a general router. Gate: `http_loop_3paths`. |

## Gates

| Wave | Gate test | Theme |
|------|-----------|--------|
| A | `emit_wasm_clang_produces_wasm` | real `.wasm` magic via `emit-wasm` |
| B | `deps_attest_write_verify_and_tamper` | local digest attest (not Rekor) |
| C | `http_loop_3paths` | triple-path bounded HTTP loop |

## Refuse

Full WASI libc, `build --target=wasm32` linking host `rt/`, browser/UI games,
fake `emit-wasm` that only writes `.ll`, CDN registry, Sigstore theater without
a digest gate, Suite5 theater claiming identical instruction work after folds.
