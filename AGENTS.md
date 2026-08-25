# Guide for AI agents working on Rynix

**Languages:** [English](AGENTS.md) (default) · [فارسی](AGENTS.fa.md)

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
  (manifest build: `docs/contracts/wave12_manifest.contract.toml`;
  Phase 19 path MCP: `docs/contracts/phase19_path_mcp.contract.toml`)
- Agent write gate: `patch --write` denied unless `rynix.scope.toml` / `--force-write`
- Path deps: `rynixc deps` → `rynix.deps.v1` (path + local `[registry]` index;
  dir-scan or sparse `index/config.json`; optional `rynix.lock.toml` via `--lock` /
  `--locked`; local digest `rynix.attest.v1.json` via `--attest` / `--attest-verify`;
  no network)
- Package compile: unity + `pkg__fn` mangling; semver `^`/`>=`; `import std::mod`; workspace `{ workspace = true }`
- Conventions: `rynixc dna` → `rynix.dna.v1` (heuristic; not “80 layers”)
- Scaffold: `rynixc new <name>` → local package; next: `rynixc build` (cwd / entry)
- Soft HTTP: one-shot + bounded loop (header / body / keep-alive / path_param) +
  HTTP-over-TLS; `import std::fs` / `std::crypto` (SHA)
- `eval`: arith/print-oriented; unsupported CallExt hard-fails (no zero-default)
- Phase 12 complete: [docs/LEAD_AHEAD.md](docs/LEAD_AHEAD.md)
- Phase 13: `emit-ll --target=wasm32-unknown-unknown`; `[build].optimize` + `--opt`/`--no-opt` ([docs/PHASE13.md](docs/PHASE13.md))
- Phase 14: `emit-wasm` → real `.wasm` via clang (no WASI / no `rt/`); `deps --attest` → `rynix.attest.v1` local digest ([docs/PHASE14.md](docs/PHASE14.md))
- Phase 15: Node runs `emit-wasm` `main` (no WASI) ([docs/PHASE15.md](docs/PHASE15.md))
- Phase 16: honesty + `path_param` HTTP + MCP path-first ([docs/PHASE16.md](docs/PHASE16.md)); Niche-10 map ([docs/adr/0013-niche-10-scorecard.md](docs/adr/0013-niche-10-scorecard.md)); Raft deferred ([docs/adr/0012-deferred-consensus.md](docs/adr/0012-deferred-consensus.md))
- Phase 20 complete: WASM host-import `env.print_i64` + package/INSTALL polish; Niche-10 certified ([docs/NICHE10.md](docs/NICHE10.md))
- Phase 21: MCP path-first remainder + match enum variants + product example ([docs/PHASE21.md](docs/PHASE21.md))
- Phase 22: inline match+return CFG fix + MCP format/compile path-first ([docs/PHASE22.md](docs/PHASE22.md))
- Phase 23: LSP refs/symbols + `Enum::Variant` + `Vec[str]` ([docs/PHASE23.md](docs/PHASE23.md))
- Phase 24: `Map[str, i64]` + product example ([docs/PHASE24.md](docs/PHASE24.md))
- **Golden path (25–30):** [docs/GOLDEN_PATH.md](docs/GOLDEN_PATH.md)
- Install: one-path clang Win/Linux — [INSTALL.md](INSTALL.md)
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
