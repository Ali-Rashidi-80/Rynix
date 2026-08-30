---
name: rynix
description: Honest agent guide for the Rynix systems language — compile with rynixc, use MCP tools and NDJSON diagnostics, never invent stub domains or End-style feature/skill keywords.
---

# Rynix — agent skill (evidence-first)

Rynix (`.ryx`) is an AI-native systems language: Zero-GC escape path, colorless
fibers, textual LLVM → clang, machine-readable diagnostics (`rynix.diag.v1`),
and **MCP** (`rynixc mcp-serve`, 19 tools).

Do **not** invent End-style `feature` / `skill` / `task` / `agent` language
keywords, UI canvas, fake TLS, or CDN registry rows. Deferred work lives in ADRs.
This file is a **Cursor Agent Skill** (docs pack) — not a language keyword.

## Canonical toolchain

```text
.ryx → rynixc check | dump-rir | emit-ll | emit-wasm | build | run | fmt | mcp-serve | lsp-serve
 | arch check | graph | slice | impact | eval | patch | verify | precheck | context
 | security | scope | deps | dna | new
```

- Diagnostics: `--error-format=json` → NDJSON `rynix.diag.v1`
- Alloc: `rynixc check file.ryx --explain-alloc --error-format=json`
- Packages: `rynix.toml` entry + `files`; `rynixc new <name>` then `rynixc build`
- Path deps: `rynixc deps` → `rynix.deps.v1`; `--lock` / `--locked`;
  `--attest` / `--attest-verify` → `rynix.attest.v1.json` (local digest, **not** Sigstore Rekor)
- WASM: `emit-ll --target=wasm32-unknown-unknown`; `emit-wasm -o out.wasm` (clang,
  no WASI / no `rt/`); Node can run arith `main` and host-import `env.print_i64` /
  `env.print` (str)
  ([docs/PHASE15.md](../../docs/PHASE15.md), [docs/NICHE10.md](../../docs/NICHE10.md),
  [docs/PHASE29.md](../../docs/PHASE29.md))
- Runtime: `--runtime=portable` (default) | `iocp` (Windows) | `uring` (Linux)
- Contracts: `rynixc verify --contract=docs/contracts/…`
- Agent write: `patch --write` needs `rynix.scope.toml` or `--force-write`

## Language pointers

- Spec: `docs/SPEC.md`
- Roadmap / phases: `docs/ROADMAP.md`, `docs/LEAD_AHEAD.md`, `docs/PHASE14.md`, `docs/PHASE15.md`,
  `docs/PHASE16.md`, **`docs/GOLDEN_PATH.md`** (Q-Core 25–29);
  **`docs/GOLDEN_REMAINING.md`** (Phases 30–38, closed);
  **`docs/GOLDEN_LEAD.md`** (Phases 39–49, active Lead SoT);
  Phase 27 security: [PHASE27.md](../../docs/PHASE27.md), ADR-0022/0023;
  Phase 28–30: [PHASE28.md](../../docs/PHASE28.md), [PHASE29.md](../../docs/PHASE29.md),
  [PHASE30.md](../../docs/PHASE30.md); Phase 38 codeAction: [PHASE38.md](../../docs/PHASE38.md)
- Contracts: `docs/contracts/wave1.contract.toml`, `wave12_manifest.contract.toml`,
  `phase19_path_mcp.contract.toml` (path-first MCP + LSP completion/rename),
  `phase21_roi.contract.toml` (MCP path-first remainder + match variants),
  `phase22_inline_mcp.contract.toml` (inline match+return + MCP format/compile),
  `phase23_depth.contract.toml` (LSP refs + Enum::Variant + Vec[str]),
  `phase24_map_str.contract.toml` (Map[str,i64] + example 12),
  `phase25_golden.contract.toml` (Map[str,str] + documentSymbol + example 13),
  `phase28_agent.contract.toml`, `phase29_ceiling.contract.toml`
- vs End verdict: `docs/VERDICT.md`, `docs/END_PEER_GAP.md` (peer **`bdc8732`** — host rustls/h2 real; MCP still absent)
- Soft builtins and std: README Soft builtins + `std/*.ryx` (`std::fs`, `std::crypto`
  SHA/HMAC/AES facades,
  HTTP loop `_2paths_` / `_3paths_` / `path_param`); `Vec[i64]` / `Vec[str]` /
  `Map[i64,i64]` / `Map[str,i64]` / `Map[str,str]` mono
- LSP (`rynixc lsp-serve`): diagnostics, hover, go-to-definition, **completion**,
  **rename** (incl. prepareRename), **references**, **documentHighlight**,
  **workspace/symbol**, **documentSymbol**, **formatting**, **codeAction**, **inlayHint**
- MCP: prefer filesystem `path` (path-first; fail-closed on missing file) for
  `rynix_graph` / `rynix_slice` / `rynix_impact` / `rynix_precheck` / `rynix_check` /
  `rynix_context` / `rynix_security` / `apply_fix` / `rynix_format` /
  `rynix_explain_alloc` / `compile` / `ast_query`; inline `source` still works;
  **19 tools**; **`server/discover`** dual-era metadata; tool annotations on graph/apply_fix
- Language: `match` on nullary enum variants + `Enum::Variant` paths
  ([ADR-0015](../../docs/adr/0015-match-enum-variants.md)); `Vec[str]`
  ([ADR-0016](../../docs/adr/0016-vec-str-mono.md)); `Map[str, i64]`
  ([ADR-0017](../../docs/adr/0017-map-str-i64-mono.md)); `Map[str, str]`
  ([ADR-0018](../../docs/adr/0018-map-str-str-mono.md)); `Option[i64]` / `Option[str]`
  ([ADR-0026](../../docs/adr/0026-option-t-mono.md))
- Memory: escape / region / linear move (`RYX2011`) / `#^ effect: pure` (`RYX2012`)
- Reserved stubs rejected: `tensor` / `signal` / `agent` → `RYX2013`

## Honesty

- Prefer fixing the compiler over loosening a test.
- Do not mark ROADMAP ✅ without in-tree tests.
- Suite5: opaque trip counts; strength reduction allowed only with matching checksums
  and disclosed Notes — not “identical work across languages.”
- Peer End clone: read-only for audit; never edit friend sources for marketing wins.
- Refuse: full WASI theater, UI canvas stubs, Raft/GGUF Stable rows without real FFI gates.
- Niche-10 path: [docs/adr/0013-niche-10-scorecard.md](../../docs/adr/0013-niche-10-scorecard.md);
  certification: [docs/NICHE10.md](../../docs/NICHE10.md);
  Raft deferred: [docs/adr/0012-deferred-consensus.md](../../docs/adr/0012-deferred-consensus.md)

## Quick compile

```sh
rynixc new hello && cd hello && rynixc build && rynixc run
rynixc check main.ryx --error-format=json
rynixc emit-wasm testdata/wasm_arith.ryx -o target/wasm_arith.wasm
rynixc deps . --attest
rynixc mcp-serve
```
