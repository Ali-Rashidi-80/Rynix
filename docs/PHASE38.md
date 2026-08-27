# Phase 38 — Agent surface remainder

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md). Optional after Phase 33;
executed after Phase 37 close.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | LSP `textDocument/codeAction` from `Diagnostic.fixes` | `lsp_code_action_smoke` | ✅ |
| B | Honest MCP tool count (=19, no ≥20 theater) | `mcp_tool_count_honest` | ✅ |
| C | Stale “18 tools” peer docs | COMPARE / END_PEER_GAP | ✅ |
| D | Contract + this document | `verify_phase38_agent_contract` | ✅ |

Contract: [contracts/phase38_agent.contract.toml](contracts/phase38_agent.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| AI tooling | 9.8 | 9.9 | `lsp_code_action_smoke` (E-9 remainder) |

## Out of scope

- `inlayHint` / `prepareRename` / `documentHighlight` → Track R
- MCP ≥20 / HTTP-SSE / `mcp_cmd` → `mcp/` split → Track R
- Parametric `Option[T]` / traits (language — not agent surface)
