# Changelog

All notable changes to Rynix are documented here. The format is inspired by
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
tracks versions informally until a tagged release is explicitly requested.

## [Unreleased]

### Added

- Phase 29: uring/TLS/HTTP honesty docs, WASM host-import `env.print` (str), Book
  skeleton (`docs/book/`), Suite5 artifact links, RFC template + CONTRIBUTING RFCs
  section, `phase29_ceiling` contract. Phase 30 remains user-triggered only.
- Phase 28: LSP `textDocument/formatting`, MCP `rynix_slice`, `std::crypto`
  HMAC/AES import facade, ADR-0024 Deferred (payload enums), peer date refresh,
  `phase28_agent` contract.
- Phase 27: `--sandbox=docker|none` (ADR-0022), RIR/sema sanitize of `system`/`exec*`
  (ADR-0023), STRIDE threat model, CWE matrix, sandbox skip/Windows deferral docs,
  `parse_no_crash` fuzz target + seed, `phase27_security` contract.
- Phase 26: `lower/` + `lsp/` decomp (ADR-0019/0020), unwrap budget gate, contract schema.
- Phase 25: `Map[str, str]` mono ([ADR-0018](docs/adr/0018-map-str-str-mono.md)).
- Phase 25: LSP `textDocument/documentSymbol`.
- Example `examples/13_http_map_str_str.ryx`.
- Phase 24: `Map[str, i64]` mono ([ADR-0017](docs/adr/0017-map-str-i64-mono.md)).
- Example `examples/12_http_vec_map_str.ryx`.

### Added (Phase 23)

- Phase 23: LSP `textDocument/references` + `workspace/symbol`.
- Phase 23: `Enum::Variant` nullary paths in exprs and `match` arms.
- Phase 23: `Vec[str]` mono (`vec_str_*`, [ADR-0016](docs/adr/0016-vec-str-mono.md)).

### Fixed

- Phase 22: inlined `match`/`if` where every arm `return`s no longer leaves an
  empty CFG join that becomes a phantom `inline_merge` predecessor (clang
  `phi` referencing undefined `%bN`). LLVM emit also skips unreachable phi preds.

### Added (earlier unreleased)

- Phase 22: MCP path-first for `rynix_format` / `rynix_explain_alloc` / `compile` /
  `ast_query`.
- Phase 21: MCP path-first for `rynix_check` / `diagnostics`, `rynix_context`,
  `rynix_security`, and `apply_fix` (fail-closed disk read; inline `source`
  still optional).
- `match` on nullary enum variant idents ([ADR-0015](docs/adr/0015-match-enum-variants.md)).
- Example `examples/11_http_path_param_tls.ryx` (path_param loop + HTTP TLS).
- `docs/PHASE21.md` / `PHASE22.md` / `PHASE23.md` + contracts.

### Changed

- `docs/GOLDEN_PATH.md`: Quality-10 absorb from analysis (8.7) + 90-day quality
  plan — Security/decompose/fuzz elevated; parametric generics → Track G;
  ADR-0018 reserved for `Map[str,str]` mono.
- README / README.fa.md: Pics2PPT-style centered header (logo, language switcher,
  badges, TOC details).
- `PRODUCTION_READINESS.md`: phases 0–22+ honesty; MCP path-first list; CodeLens;
  LSP references; `Vec[str]`.
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
