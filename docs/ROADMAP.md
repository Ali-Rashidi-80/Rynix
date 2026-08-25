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
**`match`** on int/bool/`_`/nullary enum variants (+ `else`) → `icmp`/`br` chains.
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
| Local path packages (`rynix.toml` deps) | ✅ | path + index + unity + multifile + workspace + lock + transitive + mangling + semver + `std::` |
| `rynixc new` / `dna` | ✅ | scaffold + `rynix.dna.v1` + MCP |
| TLS echo (SChannel / OpenSSL) | ✅ | `tls_echo_smoke_c` + soft builtins |
| HMAC + AES-GCM KAT | ✅ | RFC 4231 + NIST empty-tag; `crypto_kv_smoke` |
| VS Code CodeLens (check/alloc/impact) | ✅ | `editors/vscode` CodeLens provider |
| Suite12 MATCH ports | ✅ | ALU/HFT/JSON/FSM/DNA/GEMM/MC/trees/SHA checksum gates |
| Suite12 divergent #1/#5/#6 | ✅ deferred | [ADR-0011](adr/0011-suite12-divergent-benches.md) — no stub |
| WS RFC6455 (64-bit + frag + large wire) | ✅ | `ws_frames_smoke_c` + `ws_large_echo_smoke_c` |
| Windows IOCP runtime | ✅ | AcceptEx/ConnectEx + WSARecv/WSASend |
| GPG release path | ✅ | `release.yml` + `gpg_sign_smoke` |
| Release packaging + optional GPG | ✅ | `scripts/build_release.ps1` + `release.yml` |
| DCE strips dead Suite5 noise | ✅ | `dce_matrix_noise` — matrix LLVM is `opaque*216` |
| C11 backend | 🔄 deferred | ADR-0008 — no stub transpile |
| UI / canvas | 🔄 deferred | ADR-0007 — no stub studio |
| Network package registry | 🔄 deferred | ADR-0010 — local dir-scan + **sparse** index + unity + lock; no CDN |

## Phase 12 — Lead ahead (valuable, not theatrical)

Ordered plan (build-ready locks): **[LEAD_AHEAD.md](LEAD_AHEAD.md)**.
Research archive (peer + web): **[research/PHASE12_RESEARCH_INVENTORY.md](research/PHASE12_RESEARCH_INVENTORY.md)**.
Peer End `main` @ `cf5bef3` (2026-08-24) still simulates TLS/JIT/registry;
Rynix does not copy that surface. Order: `0 → 1 → (1b ∥ 2) → 3 → 4 → (5 ∥ 6)`.

| Wave | Theme | Status |
|------|--------|--------|
| 0 | Honesty freeze + field-assign reject (`RYX2020`) + README truth (shipped domains) | ✅ |
| 1 | `rynixc build`/`run` from `[package].entry`+`files` | ✅ |
| 1b | Memory `compile_fail` corpus (∥ Wave 2) | ✅ |
| 2 | Bounded looping HTTP (`max_reqs`) (∥ Wave 1b) | ✅ |
| 3 | Struct literals (i64 fields v1) + field store | ✅ |
| 4 | `import std::fs` / thin crypto SHA | ✅ |
| 5 | Suite12 MATCH as `.ryx` checksums (∥ Wave 6) | ✅ |
| 6 | LSP workspace go-to-def + eval honesty (∥ Wave 5) | ✅ |

## Phase 13 — WASM emit-ll + manifest optimize

Plan: **[PHASE13.md](PHASE13.md)**. Order: A then B.

| Wave | Theme | Status |
|------|--------|--------|
| A | `emit-ll --target=wasm32-unknown-unknown` + clang `-c` smoke | ✅ `emit_ll_wasm32_clang_accepts` |
| B | `[build].optimize` + `--opt`/`--no-opt` for build/run | ✅ `build_respects_manifest_optimize` |
| C | Dual-path HTTP loop (`http_serve_loop_2paths_json_i64`) | ✅ `http_loop_2paths` |

## Phase 14 — Real `.wasm` (clang link, no WASI)

Plan: **[PHASE14.md](PHASE14.md)**. Locked Wave A over Sigstore-lite / deeper I/O.

| Wave | Theme | Status |
|------|--------|--------|
| A | `rynixc emit-wasm` → real `.wasm` (`\0asm`) via clang | ✅ `emit_wasm_clang_produces_wasm` |
| B | Local digest attest (`rynix.attest.v1.json`, not Rekor) | ✅ `deps_attest_write_verify_and_tamper` |
| C | Triple-path HTTP loop (`http_serve_loop_3paths_json_i64`) | ✅ `http_loop_3paths` |

## Phase 15 — Run emitted `.wasm` (Node, no WASI)

Plan: **[PHASE15.md](PHASE15.md)**. Closes the emit→execute loop without WASI.

| Wave | Theme | Status |
|------|--------|--------|
| A | Node instantiates `emit-wasm` module; `main()===42` | ✅ `emit_wasm_node_runs_main` |
| B | Skills-as-docs pack (`emit-wasm` + attest honesty) | ✅ `agent_skill_mentions_emit_wasm_and_attest` |

## Phase 16 — Honesty + path_param HTTP + MCP path-first

Plan: **[PHASE16.md](PHASE16.md)**. Niche-10 base ([ADR-0013](adr/0013-niche-10-scorecard.md));
Raft deferred ([ADR-0012](adr/0012-deferred-consensus.md)). Order: `0→A→B→C→D`.

| Wave | Theme | Status |
|------|--------|--------|
| 0 | PHASE16 + ADR-0012/0013 locks | ✅ |
| A | Suite5 + peer refresh | ✅ Suite5 Phase 16-A artifact |
| B | PRODUCTION_READINESS / local-digest wording | ✅ |
| C | `http_serve_loop_path_param_json_i64` | ✅ `http_loop_path_param` |
| D | MCP `rynix_graph` path-first | ✅ `mcp_graph_path_file` |

## Phases 17–20 — Niche-10 path (after 16)

| Phase | Theme | Status |
|-------|--------|--------|
| 17 | Language: struct str, index assign, enum values, ADR-0014 mono collections | ✅ `struct_str_field_roundtrip`, `index_assign_ok`, `enum_value_roundtrip` |
| 18 | Product HTTP(+TLS): header, bounded body, keep-alive, TLS path | ✅ `http_header_i64_smoke`, `http_body_bounded_smoke`, `http_keepalive_bounded_smoke`, `http_tls_product_smoke` |
| 19 | LSP completion+rename; MCP path-first trio; agent contract | ✅ `completion_lists_fn_and_let`, `rename_local_updates_def_and_refs`, `mcp_impact_path_file`, `mcp_precheck_path_file`, `verify_phase19_path_mcp_contract` |
| 20 | WASM host-import `print_i64`; package/INSTALL polish; Niche-10 certify | ✅ `emit_wasm_host_print_i64`, `package_ux_new_deps_attest`, `install_one_path_clang_win_linux`, `niche10_scorecard_links_gates` — [NICHE10.md](NICHE10.md) |

## Phase 21 — ROI after Niche-10

Plan: **[PHASE21.md](PHASE21.md)**. MCP path-first remainder, product example,
match on enum variants ([ADR-0015](adr/0015-match-enum-variants.md)), CHANGELOG,
VS Code completion/rename docs.

| Wave | Theme | Status |
|------|--------|--------|
| Hyg | README headers + CodeLens honesty | ✅ |
| A | MCP `check` / `context` / `security` / `apply_fix` path-first | ✅ `mcp_*_path_file` |
| B | `examples/11_http_path_param_tls.ryx` | ✅ `example_http_path_param_tls_checks` |
| C | `match` nullary enum variants | ✅ `enum_match_variant_roundtrip` |
| D | CHANGELOG (no push unless asked) | ✅ |
| E | VS Code LSP completion/rename client honesty | ✅ |

## Phase 22 — Inline match+return + MCP path-first remainder

Plan: **[PHASE22.md](PHASE22.md)**. Fix phantom CFG join after exhaustive
`return` in match/if when inlined; finish MCP disk-first tools.

| Wave | Theme | Status |
|------|--------|--------|
| A | Empty join → unreachable; reachable-only phi | ✅ `inline_match_return_roundtrip` |
| B | MCP `format` / `explain_alloc` / `compile` / `ast_query` path-first | ✅ `mcp_format_path_file`, `mcp_compile_path_file` |

## Phase 23 — Depth: LSP refs, Enum::Variant, Vec[str], tag

Plan: **[PHASE23.md](PHASE23.md)**.

| Wave | Theme | Status |
|------|--------|--------|
| Hyg | PRODUCTION_READINESS honesty (0–22+) | ✅ |
| A | LSP references + workspace/symbol | ✅ `references_lists_local_uses`, `workspace_symbol_lists_fn` |
| B | `Enum::Variant` paths + match | ✅ `enum_qualified_variant_roundtrip` |
| C | `Vec[str]` mono ([ADR-0016](adr/0016-vec-str-mono.md)) | ✅ `vec_str_roundtrip` |
| D | Local tag `v0.1.0` (no push) | ✅ |

## Phase 24 — Map[str,i64] + product example

Plan: **[PHASE24.md](PHASE24.md)**.

| Wave | Theme | Status |
|------|--------|--------|
| A | `Map[str, i64]` mono ([ADR-0017](adr/0017-map-str-i64-mono.md)) | ✅ `map_str_i64_roundtrip` |
| B | `examples/12_http_vec_map_str.ryx` | ✅ `example_http_vec_map_str_checks` |

## Golden path (post-24)

Canonical next plan: **[GOLDEN_PATH.md](GOLDEN_PATH.md)** — Phases **25–30**,
full backlog map, refuse set, DoD. Default execute: Phase 25 (`Map[str,str]` +
`documentSymbol`).

## Follow-on (post-13 / parallel)

| Item | Status | Gate |
|------|--------|------|
| Local sparse package index (no CDN) | ✅ | `deps_resolves_sparse_local_index`, `build_pkg_sparse_app_resolves_index` |
| Suite5 `sum` closed form without `sdiv`/IDIV | ✅ | `sum_opaque_closed_form_has_no_sdiv_or_loop` |
| Local package digest attest (not Sigstore/Rekor) | ✅ `rynix.attest.v1` | `deps_attest_write_verify_and_tamper` |
| HTTP ≥3 paths / extra fiber smokes | ✅ | `http_loop_3paths` |

## CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml): `cargo test` on
Ubuntu + Windows; clippy; VS Code extension compile; Suite5 + arch gates;
ASan fiber/echo/TCP/load/json; uring SQE + uring TCP smoke on Linux;
optional `suite5-with-end` when `endc` on PATH; `size_echo_gates` suite12 +
WS/IOCP/TLS/crypto smokes; release GPG smoke (`gpg_detach_sign_smoke`).
