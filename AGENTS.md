# Guide for AI agents working on Rynix

Rynix is an AI-native systems language. Prefer machine-readable surfaces over
ad-hoc scraping.

## Canonical toolchain

```text
.ryx → rynixc check | dump-rir | emit-ll | build | run | fmt | mcp-serve | lsp-serve
     | arch check | graph | slice | impact | eval | patch
```

- Diagnostics: `--error-format=json` → NDJSON `rynix.diag.v1`
  (schema: `docs/schemas/rynix.diag.v1.json`)
- Alloc transparency: `rynixc check file.ryx --explain-alloc --error-format=json`
- MCP: `rynixc mcp-serve` (11 tools: graph, impact, eval, arch, …)
- Structure: `rynixc graph file.ryx` / `rynixc slice file.ryx`

## Honesty rules

- Do not mark ROADMAP items ✅ without in-tree tests.
- Do not widen language surface in docs without SPEC + tests.
- Prefer fixing the compiler over loosening a test.
- Windows uses `--runtime=portable`; Linux may use `--runtime=uring`.

## Layout

See [README.md](README.md) and [docs/ROADMAP.md](docs/ROADMAP.md).
Irreversible decisions live in [docs/adr/](docs/adr/).
