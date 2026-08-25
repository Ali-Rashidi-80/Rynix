# Changelog

All notable changes to Rynix are documented here. The format is inspired by
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
tracks versions informally until a tagged release is explicitly requested.

## [Unreleased]

### Fixed

- Phase 22: inlined `match`/`if` where every arm `return`s no longer leaves an
  empty CFG join that becomes a phantom `inline_merge` predecessor (clang
  `phi` referencing undefined `%bN`). LLVM emit also skips unreachable phi preds.

### Added

- Phase 22: MCP path-first for `rynix_format` / `rynix_explain_alloc` / `compile` /
  `ast_query`.
- Phase 21: MCP path-first for `rynix_check` / `diagnostics`, `rynix_context`,
  `rynix_security`, and `apply_fix` (fail-closed disk read; inline `source`
  still optional).
- `match` on nullary enum variant idents ([ADR-0015](docs/adr/0015-match-enum-variants.md)).
- Example `examples/11_http_path_param_tls.ryx` (path_param loop + HTTP TLS).
- `docs/PHASE21.md` + `docs/contracts/phase21_roi.contract.toml`.
- `docs/PHASE22.md` + `docs/contracts/phase22_inline_mcp.contract.toml`.

### Changed

- README / README.fa.md: Pics2PPT-style centered header (logo, language switcher,
  badges, TOC details).
- `PRODUCTION_READINESS.md`: documents VS Code CodeLens (was incorrectly “No CodeLens”).
- VS Code extension docs: LSP completion + rename via LanguageClient.

## [0.1.0] — Niche-10 certified (Phases 16–20)

Local tree certification — **not** Absolute-10 vs Go. See [docs/NICHE10.md](docs/NICHE10.md).

### Highlights

- Suite5 honesty deepen; `http_serve_loop_path_param_json_i64`
- Product HTTP: header / bounded body / keep-alive / TLS path
- Language: struct `str`, index assign, nullary enum values
- MCP path-first: `rynix_graph` / `rynix_impact` / `rynix_precheck`
- LSP completion + rename; WASM host-import `env.print_i64`
- Package UX + INSTALL one-path clang; local `rynix.attest.v1` digest

[Unreleased]: https://github.com/rynix-lang/rynix/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rynix-lang/rynix/releases/tag/v0.1.0
