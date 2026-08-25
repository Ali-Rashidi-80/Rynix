# Phase 23 — Hygiene + LSP refs + Enum.Variant + Vec[str] + tag

Post Phase 22. Order: docs honesty → LSP → language → collections → local tag.

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| Hyg | `PRODUCTION_READINESS` phases 0–22 + MCP path-first list | file contains `Phase 22` / path-first tools |
| A | LSP `textDocument/references` + `workspace/symbol` | `references_lists_local_uses`, `workspace_symbol_lists_fn` |
| B | `Enum.Variant` / `Enum::Variant` nullary paths ([ADR-0015](adr/0015-match-enum-variants.md) amend) | `enum_qualified_variant_roundtrip` |
| C | `Vec[str]` mono ([ADR-0016](adr/0016-vec-str-mono.md)) | `vec_str_roundtrip` |
| D | Local git tag `v0.1.0` (no push) | tag exists |

Contract: [contracts/phase23_depth.contract.toml](contracts/phase23_depth.contract.toml).

## Out of scope

- Payload enum match (`Some(T)`)
- Parametric `Vec[T]` / CDN / Raft / UI / full WASI
- Pushing tags or GitHub Releases
