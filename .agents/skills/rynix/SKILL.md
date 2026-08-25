---
name: rynix
description: Honest agent guide for the Rynix systems language — compile with rynixc, use MCP tools and NDJSON diagnostics, never invent stub domains or End-style feature/skill keywords.
---

# Rynix — agent skill (evidence-first)

Rynix (`.ryx`) is an AI-native systems language: Zero-GC escape path, colorless
fibers, textual LLVM → clang, machine-readable diagnostics (`rynix.diag.v1`),
and **MCP** (`rynixc mcp-serve`, 18 tools).

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
  no WASI / no `rt/`); Node can run arith `main` ([docs/PHASE15.md](../../docs/PHASE15.md))
- Runtime: `--runtime=portable` (default) | `iocp` (Windows) | `uring` (Linux)
- Contracts: `rynixc verify --contract=docs/contracts/…`
- Agent write: `patch --write` needs `rynix.scope.toml` or `--force-write`

## Language pointers

- Spec: `docs/SPEC.md`
- Roadmap / phases: `docs/ROADMAP.md`, `docs/LEAD_AHEAD.md`, `docs/PHASE14.md`, `docs/PHASE15.md`
- vs End verdict: `docs/VERDICT.md`, `docs/END_PEER_GAP.md`
- Soft builtins and std: README Soft builtins + `std/*.ryx` (`std::fs`, `std::crypto`,
  HTTP loop `_2paths_` / `_3paths_`)
- Memory: escape / region / linear move (`RYX2011`) / `#^ effect: pure` (`RYX2012`)
- Reserved stubs rejected: `tensor` / `signal` / `agent` → `RYX2013`

## Honesty

- Prefer fixing the compiler over loosening a test.
- Do not mark ROADMAP ✅ without in-tree tests.
- Suite5: opaque trip counts; strength reduction allowed only with matching checksums
  and disclosed Notes — not “identical work across languages.”
- Peer End clone: read-only for audit; never edit friend sources for marketing wins.
- Refuse: full WASI theater, UI canvas stubs, Raft/GGUF Stable rows without real FFI gates.

## Quick compile

```sh
rynixc new hello && cd hello && rynixc build && rynixc run
rynixc check main.ryx --error-format=json
rynixc emit-wasm testdata/wasm_arith.ryx -o target/wasm_arith.wasm
rynixc deps . --attest
rynixc mcp-serve
```
