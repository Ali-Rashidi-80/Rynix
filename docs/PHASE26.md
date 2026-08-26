# Phase 26 — Maturity decompose (Quality-10)

Parent: [GOLDEN_PATH.md](GOLDEN_PATH.md). No language surface widen except process/docs.

## Waves

| Wave | Theme | Gate |
|------|--------|------|
| A | Split `lower.rs` → `lower/` ([ADR-0019](adr/0019-lower-decomp.md)) | `lower_decomp_invariants` |
| B | Split `lsp_cmd` → `lsp/` ([ADR-0020](adr/0020-lsp-decomp.md)) | `lsp_decomp_parity` |
| C | unwrap/expect budget ≤ 60 in `crates/*/src` | `unwrap_budget_gate` |
| D | Repository URL honesty | `repo_url_real` *or* documented deferral |
| E | Phase-contract schema ([ADR-0021](adr/0021-phase-contract-schema.md)) | `contract_schema_gate` |
| F | Sanitizer CI scaffold documented | `sanitizer_scaffold_documented` |
| G | `cargo deny` or deferral | `cargo_deny_or_deferral` |

Contract: [contracts/phase26_maturity.contract.toml](contracts/phase26_maturity.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Rust code quality | 8.6 | 9.2 | lower/lsp decomp + unwrap budget |

## Out of scope

- lsp-types migration (Track R)
- Language surface, push/release
