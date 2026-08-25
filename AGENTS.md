# Guide for AI agents working on Rynix

Rynix is an AI-native systems language. Prefer machine-readable surfaces over
ad-hoc scraping.

## Canonical toolchain

```text
.ryx → rynixc check | dump-rir | emit-ll | emit-wasm | build | run | fmt | mcp-serve | lsp-serve
 | arch check | graph | slice | impact | eval | patch | verify | precheck | context
 | security | scope | deps | dna | new
```

- Diagnostics: `--error-format=json` → NDJSON `rynix.diag.v1`
  (schema: `docs/schemas/rynix.diag.v1.json`)
- Alloc transparency: `rynixc check file.ryx --explain-alloc --error-format=json`
- MCP: `rynixc mcp-serve` (18 tools: graph, impact, eval, arch, verify, precheck, context, security, scope, deps, dna, …)
- Structure: `rynixc graph file.ryx` / `rynixc slice file.ryx`
- Contracts: `rynixc verify --contract=docs/contracts/wave1.contract.toml`
  (manifest build: `docs/contracts/wave12_manifest.contract.toml`)
- Agent write gate: `patch --write` denied unless `rynix.scope.toml` / `--force-write`
- Path deps: `rynixc deps` → `rynix.deps.v1` (path + local `[registry]` index;
  dir-scan or sparse `index/config.json`; optional `rynix.lock.toml` via `--lock` /
  `--locked`; no network)
- Package compile: unity + `pkg__fn` mangling; semver `^`/`>=`; `import std::mod`; workspace `{ workspace = true }`
- Conventions: `rynixc dna` → `rynix.dna.v1` (heuristic; not “80 layers”)
- Scaffold: `rynixc new <name>` → local package; next: `rynixc build` (cwd / entry)
- Soft HTTP: one-shot + bounded loop; `import std::fs` / `std::crypto` (SHA)
- `eval`: arith/print-oriented; unsupported CallExt hard-fails (no zero-default)
- Phase 12 complete: [docs/LEAD_AHEAD.md](docs/LEAD_AHEAD.md)
- Phase 13: `emit-ll --target=wasm32-unknown-unknown`; `[build].optimize` + `--opt`/`--no-opt` ([docs/PHASE13.md](docs/PHASE13.md))
- Phase 14: `emit-wasm` → real `.wasm` via clang (no WASI / no `rt/`) ([docs/PHASE14.md](docs/PHASE14.md))
- Peer verdict (who is ahead?): [docs/VERDICT.md](docs/VERDICT.md)

## Honesty rules

- Do not mark ROADMAP items ✅ without in-tree tests.
- Do not widen language surface in docs without SPEC + tests.
- Prefer fixing the compiler over loosening a test.
- Windows: `--runtime=portable` (default) or `--runtime=iocp` (AcceptEx/ConnectEx);
  Linux may use `--runtime=uring`.
- Suite5: opaque trip counts block literal fold; strength reduction is allowed only when
  checksums match and docs/Notes disclose it. Do not claim identical instruction work
  across languages after reductions.

## Layout

See [README.md](README.md) and [docs/ROADMAP.md](docs/ROADMAP.md).
Irreversible decisions live in [docs/adr/](docs/adr/).
License: [LICENSE.md](LICENSE.md) (MIT OR Apache-2.0).
