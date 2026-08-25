# Phase 21 — Hygiene leftovers + ROI waves (post Niche-10)

Plan after Phases 16–20 certification. Prefer shipping surfaces over theater.
No Absolute-10 vs Go; no Raft/UI/full WASI.

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| Hyg | README Pics2PPT headers; PRODUCTION_READINESS CodeLens honesty | committed |
| A | MCP path-first: `rynix_check` / `rynix_context` / `rynix_security` / `apply_fix` | `mcp_check_path_file`, `mcp_context_path_file`, `mcp_security_path_file`, `mcp_apply_fix_path_file` |
| B | `examples/11_http_path_param_tls.ryx` product surface | `example_http_path_param_tls_checks` |
| C | `match` on nullary enum variant paths ([ADR-0015](adr/0015-match-enum-variants.md)) | `enum_match_variant_roundtrip` (prints `2`) |
| D | `CHANGELOG.md` (tag/push only on explicit request) | file present |
| E | VS Code client docs: LSP completion/rename via LanguageClient | `editors/vscode` README + package.json |

Contract: [contracts/phase21_roi.contract.toml](contracts/phase21_roi.contract.toml).

## Out of scope

- `Vec[str]` (stays ADR-0014 deferral)
- Payload enum match / `Enum.Variant` qualified paths
- Pushing git tags or GitHub Releases without an explicit ask
