# Cargo deny deferral (Phase 26-G)

**Status:** Deferred with honesty gate `cargo_deny_or_deferral`.

`cargo-deny` (advisories + licenses) is valuable supply-chain hygiene (HTML B-8)
but is **not** required to lock Quality-10 Security ≥9.0 (sandbox + sanitize in
Phase 27). Enable when CI wants it; until then this file documents the deferral.

Revisit: add `deny.toml` + CI job, then retire this deferral note.
