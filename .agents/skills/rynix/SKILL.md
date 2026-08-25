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

## Canonical toolchain

```text
.ryx → rynixc check | dump-rir | emit-ll | build | run | fmt | mcp-serve | lsp-serve
 | arch check | graph | slice | impact | eval | patch | verify | precheck | context
 | security | scope | deps | dna | new
```

- Diagnostics: `--error-format=json` → NDJSON `rynix.diag.v1`
- Alloc: `rynixc check file.ryx --explain-alloc --error-format=json`
- Packages: `rynix.toml` entry + `files`; `rynixc new <name>` then `rynixc build`
- Runtime: `--runtime=portable` (default) | `iocp` (Windows) | `uring` (Linux)
- Contracts: `rynixc verify --contract=docs/contracts/…`
- Agent write: `patch --write` needs `rynix.scope.toml` or `--force-write`

## Language pointers

- Spec: `docs/SPEC.md`
- Roadmap / Phase 12: `docs/ROADMAP.md`, `docs/LEAD_AHEAD.md`
- vs End verdict: `docs/VERDICT.md`, `docs/END_PEER_GAP.md`
- Soft builtins and std: README Soft builtins + `std/*.ryx` (`std::fs`, `std::crypto` SHA)
- Memory: escape / region / linear move (`RYX2011`) / `#^ effect: pure` (`RYX2012`)
- Reserved stubs rejected: `tensor` / `signal` / `agent` → `RYX2013`

## Honesty

- Prefer fixing the compiler over loosening a test.
- Do not mark ROADMAP ✅ without in-tree tests.
- Suite5: opaque trip counts; strength reduction allowed only with matching checksums
  and disclosed Notes — not “identical work across languages.”
- Peer End clone: read-only for audit; never edit friend sources for marketing wins.

## Quick compile

```sh
rynixc new hello && cd hello && rynixc build && rynixc run
rynixc check main.ryx --error-format=json
rynixc mcp-serve
```
