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
- **IOCP (Windows):** `--runtime=iocp` — AcceptEx/ConnectEx +
  WSARecv/WSASend; fiber park/unpark; `iocp_echo_smoke`
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
| End peer benchmarks + gap closure | ✅ Phase 11 | [END_PEER_GAP.md](END_PEER_GAP.md) |
| UI / hot-reload / canvas frameworks | 🔄 out of scope v0.1 | [ADR-0007](adr/0007-deferred-ui-frameworks.md) |

## Phase 11 — Peer parity vs End ✅

Close product gaps **without** copying End prose or claiming unverified ✅.
Gap analysis: [END_PEER_GAP.md](END_PEER_GAP.md).
**Ordered execution plan:** [SURPASS_END_PLAN.md](SURPASS_END_PLAN.md)
(Phases A→E: language → agent CLI → stdlib → runtime/packages → polish).

| Item | Status | Evidence |
|------|--------|----------|
| Suite5 `end` builder + 12× `.end` ports | ✅ | `benchmarks/suite5/*.end`, checksums validated vs C |
| End checksum validation (local) | ✅ | `endc` + all 12 OK; CI `suite5-with-end` when present |
| End-style link flags (`-flto`, `-funroll-loops`, strip) | ✅ | `build_cmd.rs`; Suite5 rebench |
| `--bench` minimal RT (size vs End) | ✅ | `rt/bench_rt.c` + const-print C; ~18 KiB MSVCRT |
| README domain maturity matrix | ✅ | root `README.md` |
| Agent contracts approach | ✅ design | [ADR-0009](adr/0009-agent-contracts-toolchain.md) |
| Close End wins on Suite5 rows | ✅ local | opaque bounds + disclosed strength reduction; see END_PEER_GAP |
| Broader HTTP / frameworks | ✅ C1–C5 landed | serve/post/echo, sha256, kv, TLS, WS; ADR-0007 for UI |
| Agent verify / precheck / context | ✅ | `verify`/`precheck`/`context` + schemas + MCP |
| Explicit `region` scopes | ✅ | SPEC §3.1 + `examples/08_region.ryx` |
| Pipeline `|>` | ✅ | SPEC §3.2 + `pipe_desugar` + `examples/09_pipe.ryx` |
| Use-after-move (linear types) | ✅ | `RYX2011` + `sema_unit` |
| `#^ effect: pure` | ✅ | `RYX2012` + `effects_pure` + wave1 contract |
| Agent `security` / `scope` | ✅ | `agent_cli` + MCP `rynix_security` / `rynix_scope` |
| Local path packages (`rynix.toml` deps) | ✅ | path + **local index** (`[registry]`); ADR-0010 |
| `rynixc new` / `dna` | ✅ | scaffold + `rynix.dna.v1` + MCP |
| TLS echo (SChannel / OpenSSL) | ✅ | `tls_echo_smoke_c` + soft builtins |
| HMAC + AES-GCM KAT | ✅ | RFC 4231 + NIST empty-tag; `crypto_kv_smoke` |
| VS Code CodeLens (check/alloc/impact) | ✅ | `editors/vscode` CodeLens provider |
| Suite12 MATCH ports | ✅ | ALU/HFT/JSON/FSM/DNA/GEMM/MC/trees/SHA checksum gates |
| WS RFC6455 (64-bit + frag) | ✅ | `ws_frames_smoke_c` KATs |
| Windows IOCP runtime | ✅ | AcceptEx/ConnectEx + WSARecv/WSASend |
| GPG release path | ✅ | `release.yml` + `gpg_sign_smoke` |
| Release packaging + optional GPG | ✅ | `scripts/build_release.ps1` + `release.yml` |
| DCE strips dead Suite5 noise | ✅ | `dce_matrix_noise` — matrix LLVM is `opaque*216` |
| C11 backend | 🔄 deferred | ADR-0008 — no stub transpile |
| UI / canvas | 🔄 deferred | ADR-0007 — no stub studio |
| Network package registry | 🔄 deferred | ADR-0010 — local index only |

## CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml): `cargo test` on
Ubuntu + Windows; clippy; VS Code extension compile; Suite5 + arch gates;
ASan fiber/echo/TCP/load/json; uring SQE + uring TCP smoke on Linux;
optional `suite5-with-end` when `endc` on PATH; `size_echo_gates` suite12 +
WS/IOCP/TLS/crypto smokes; release GPG smoke (`gpg_detach_sign_smoke`).
