# Guide for AI agents working on Rynix

Rynix is an AI-native systems language. Prefer machine-readable surfaces over
ad-hoc scraping.

## Canonical toolchain

```text
.ryx → rynixc check | dump-rir | emit-ll | build | run | fmt | mcp-serve | lsp-serve
     | arch check | graph | slice | impact | eval | patch | verify | precheck | context
     | security | scope | deps | dna | new
```

- Diagnostics: `--error-format=json` → NDJSON `rynix.diag.v1`
  (schema: `docs/schemas/rynix.diag.v1.json`)
- Alloc transparency: `rynixc check file.ryx --explain-alloc --error-format=json`
- MCP: `rynixc mcp-serve` (18 tools: graph, impact, eval, arch, verify, precheck, context, security, scope, deps, dna, …)
- Structure: `rynixc graph file.ryx` / `rynixc slice file.ryx`
- Contracts: `rynixc verify --contract=docs/contracts/wave1.contract.toml`
- Agent write gate: `patch --write` denied unless `rynix.scope.toml` / `--force-write`
- Path deps: `rynixc deps` → `rynix.deps.v1` (path + local `[registry]` index; no network)
- Package compile: `rynixc build` unity-compiles dep entries (transitive; SPEC §6.3–6.4 `import pkg.fn`)
- Conventions: `rynixc dna` → `rynix.dna.v1` (heuristic; not “80 layers”)
- Scaffold: `rynixc new <name>` → local package (no registry CDN)

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
