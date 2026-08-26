# Phase 28 — Agent polish + language depth (Quality-10)

Parent: [GOLDEN_PATH.md](GOLDEN_PATH.md).

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | LSP `textDocument/formatting` → `format_module` / `rynixc fmt` | `lsp_formatting_applies_fmt` | ✅ |
| B | MCP `rynix_slice` (CLI `slice` parity, path-first) | `mcp_slice_or_documented_absence` | ✅ |
| C | `std::crypto` HMAC/AES import facade (soft remains ABI) | `std_crypto_hmac_aes_import_ok` | ✅ |
| D | ADR-0024 payload enums | file Status = **Deferred** | ✅ |
| E | Payload match (`Some(T)`) | skipped (0024 Deferred) | skip |
| F | Struct `bool` field | skipped (not quick; needs codegen) | skip — see Out |
| G | VERDICT / END_PEER_GAP peer date refresh | `verdict_peer_date_current` | ✅ |
| H | Multiline strings | skipped | skip — see Out |

Contract: [contracts/phase28_agent.contract.toml](contracts/phase28_agent.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| AI tooling | 9.4 | 9.6 | formatting + MCP slice |

## Out of scope / deferred notes

- **28-E:** hard-stop while [ADR-0024](adr/0024-payload-enums.md) is Deferred — no `Some` stub.
- **28-F:** struct `bool` fields not landed; literals remain `i64`/`str` only (Phase 17).
- **28-H:** multiline string syntax not added; stay with single-line `str` literals.
- MCP HTTP/SSE, Absolute-10, parametric collections, codeAction/inlayHints.
- **Phase 30** remains **user-triggered only** (no auto push/tag/release). See [GOLDEN_PATH.md](GOLDEN_PATH.md) § Phase 30.
