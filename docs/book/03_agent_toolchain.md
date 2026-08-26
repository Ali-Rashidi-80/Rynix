# 3. Agent toolchain

Machine-readable surfaces beat scraping. Spec: [SPEC.md](../SPEC.md).

| Surface | Command / note |
|---------|----------------|
| Diagnostics | `rynixc check --error-format=json` → `rynix.diag.v1` |
| Structure | `rynixc graph` / `slice` / `impact` |
| MCP | `rynixc mcp-serve` (path-first tools, including `rynix_slice`) |
| LSP | `rynixc lsp-serve` (diagnostics, hover, def, completion, rename, refs, symbols, **formatting**) |
| Contracts | `rynixc verify --contract=…` |

Agent guide: [AGENTS.md](../../AGENTS.md). Skill: `.agents/skills/rynix/SKILL.md`.
Examples: [examples/](../../examples/).

Next: [Tutorial outline](04_tutorial_outline.md).
