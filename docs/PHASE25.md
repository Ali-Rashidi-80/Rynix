# Phase 25 — Map[str,str] + documentSymbol + Quality lock

Quality-10 Wave 0 lock + collection mono + LSP outline. Parent:
[GOLDEN_PATH.md](GOLDEN_PATH.md).

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| 0 | GOLDEN_PATH Quality-10 lock | docs contain “Quality-10” |
| A | `Map[str, str]` mono ([ADR-0018](adr/0018-map-str-str-mono.md)) | `map_str_str_roundtrip` |
| B | LSP `textDocument/documentSymbol` (+ VS Code capability note) | `document_symbol_lists_fn` |
| C | `examples/13_http_map_str_str.ryx` headers-shaped demo | `example_map_str_str_product_checks` |
| D | Contract + skill/AGENTS/CHANGELOG/ROADMAP | `verify_phase25_golden_contract` |

Contract: [contracts/phase25_golden.contract.toml](contracts/phase25_golden.contract.toml).

## Axis note (Quality-10)

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| AI tooling | 9.4 | 9.5 | `document_symbol_lists_fn` |

## Out of scope

- Parametric `Map[K,V]` / ADR-0018-as-generics (Track G / R13–R16)
- Payload enums, push/release
- Phase 26+ decompose/sandbox
