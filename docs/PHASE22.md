# Phase 22 — Inline match+return fix + MCP path-first remainder

After Phase 21. Compiler correctness first; then finish MCP disk-first tools.

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| A | CFG join: empty match/if join → `unreachable` (no phantom inline_merge pred); LLVM phi only from reachable preds | `inline_match_return_roundtrip` |
| B | MCP path-first: `rynix_format` / `rynix_explain_alloc` / `compile` / `ast_query` | `mcp_format_path_file`, `mcp_compile_path_file` |

Contract: [contracts/phase22_inline_mcp.contract.toml](contracts/phase22_inline_mcp.contract.toml).

## Out of scope

- `Vec[str]` / payload enum match
- Tag/push release
