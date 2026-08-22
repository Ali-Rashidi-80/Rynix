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

---

## Phase 0 — Workspace ✅

Cargo workspace, pedantic lints, `unsafe_op_in_unsafe_fn = deny`, ADRs 0001–0006.

## Phase 1 — Lexer ✅

Zero-alloc cursor, `RYX0001..0006`, mmap, proptest tiling, zero-alloc proof,
criterion (~388 MiB/s mixed). Shipping gate ≥ ~400 MiB/s (within noise) — met.
See [benchmarks.md](benchmarks.md).

## Phase 2 — Parser / AST ✅

Arena AST, Pratt, recovery, dumps; formatter in Phase 9.

## Phase 3 — Diagnostics ✅

Human + `rynix.diag.v1` JSON schema; MCP tools completed Phase 9.

## Phase 4 — Sema ✅

Scopes, types, `#^ error`, soft std + `tensor`/`signal`/`agent`.

## Phase 5 — RIR ✅

SoA SSA, block args, DCE/const-fold/simplify-cfg/BCE, interpreter.
**Indexing** with `bounds_check` / `load_index`.
**Immutable lets** bind as SSA values; **`let mut`** stays alloc/load/store.
**`for` / `break` / `continue`** lower to real CFG.
**Field** access via `field_offsets` + `gep_i64`.
**`match`** on int/bool/`_` (+ `else`) → `icmp`/`br` chains.
**Methods** `.len` / `.push` / `.get` / `.insert` by receiver kind.

## Phase 6 — Escape ✅

Lattice, region inject, `Free { site, ptr }` → `rynix_rt_heap_free`.
`#^ alloc:` + `#^ free-at` covered.

## Phase 7 — LLVM ✅

Textual `.ll` shipping backend ([ADR-0005](adr/0005-textual-llvm-ir-first.md)),
reachability DCE, heap free codegen, hello size gate `<300KiB` (when clang
present). Differential: `diff_llvm_vs_interp`.

## Phase 8 — Runtime ✅

- Fibers (Win32 / ucontext) with **PARKED** waits for I/O
- **TCP:** listen/accept/connect/recv/send (fiber-safe)
- **Echo + RPS:** `tcp_echo_rps` + `load_harness`; methodology in
  [bakeoff.md](bakeoff.md) (+ optional Go script)
- **io_uring (Linux):** fiber-aware submit → park → CQ harvest in
  `rynix_rt_run`; APIs for read/write/**accept**/**connect**;
  `tcp_accept`/`tcp_connect` use uring when the ring is ready (else poll);
  `tcp_recv`/`tcp_send` stay non-blocking poll+yield
- Pipe echo + fiber smoke + ASan CI; `uring_sqe_smoke`

## Phase 9 — Std / tooling / AI ✅

- CLI: build/run/test/fmt/mcp-serve · `rynix.toml`
- MCP: diagnostics, format, explain_alloc, compile, ast_query, apply_fix
- **Vec/Map:** [ADR-0006](adr/0006-monomorphized-collections.md) —
  `Vec[i64]` / `Map[i64, i64]` + soft builtins + methods (complete shipping
  collections design)
- Soft net/time/io · Presburger-lite BCE · smart primitives

---

## Milestones

| Id | Criterion | Status |
|----|-----------|--------|
| M0–M6 | scaffold → escape | ✅ |
| M7 | native LLVM + size gate | ✅ (clang) |
| M8 | fiber TCP + RPS + fiber-aware uring | ✅ |
| M9 | std + MCP + fmt + BCE + mono Vec/Map | ✅ |
| M10 | product surface (LSP, Suite5 publish, richer AI CLI, install UX) | ✅ |

## Phase 10 — Competitive product surface ✅

| Item | Status | Evidence |
|------|--------|----------|
| AI CLI (`graph`/`slice`/`impact`/`eval`/`patch`) | ✅ | `crates/rynixc/tests/agent_cli.rs`, MCP tools |
| JSON schemas (graph/impact/eval) | ✅ | `docs/schemas/rynix.*.v1.json` |
| Suite5 cross-lang + checksum (12 workloads) | ✅ | `benchmarks/suite5/`, CI `suite5-check` |
| Install UX | ✅ | `install.ps1`, `INSTALL.sh`, `INSTALL.md` |
| PRODUCTION_READINESS / SECURITY | ✅ | root docs |
| Editor — LSP + VS Code | ✅ | `rynixc lsp-serve`, `editors/vscode/`, `lsp_cmd` unit test |
| `arch check` + Architecture.toml | ✅ | root `Architecture.toml`, `crates/rynixc/tests/phase10_gates.rs`, CI |
| Std json/http in `.ryx` | ✅ | `json_get_i64`, `http_get_json_i64`, `examples/05_http_json.ryx` |
| GitHub Release binaries | ✅ | `.github/workflows/release.yml` + SHA256SUMS |
| Optional C11 backend | 🔄 deferred | [ADR-0008](adr/0008-deferred-c11-backend.md) |
| End peer benchmarks + gap closure | 🔄 Phase 11 | [END_PEER_GAP.md](END_PEER_GAP.md) |
| UI / hot-reload / canvas frameworks | 🔄 out of scope v0.1 | [ADR-0007](adr/0007-deferred-ui-frameworks.md) |

## Phase 11 — Peer parity vs End (in progress)

Close product gaps **without** copying End prose or claiming unverified ✅.
Full backlog: [END_PEER_GAP.md](END_PEER_GAP.md).

| Item | Status | Evidence |
|------|--------|----------|
| Suite5 `end` builder + 12× `.end` ports | ✅ | `benchmarks/suite5/*.end`, checksums validated vs C |
| End checksum validation (local) | ✅ | `endc` + all 12 OK (CI optional) |
| README domain maturity matrix | ✅ | root `README.md` |
| Agent contracts approach | ✅ design | [ADR-0009](adr/0009-agent-contracts-toolchain.md) |
| Broader HTTP / frameworks | ⬜ | ADR-0007 |
| C11 backend | 🔄 deferred | ADR-0008 |

## CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml): `cargo test` on
Ubuntu + Windows; clippy; VS Code extension compile; Suite5 + arch gates;
ASan fiber/echo/TCP/load/json; uring SQE + uring TCP smoke on Linux.
