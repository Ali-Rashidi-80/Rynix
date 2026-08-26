# ADR-0021: Phase contract schema gate

## Status

Accepted (Phase 26)

## Context

Quality-10 wants one-phase-per-PR discipline without inventing End-style theater.
Existing contracts under `docs/contracts/*.toml` are already verified by
`rynixc verify`.

## Decision

- Keep TOML contracts as SoT evidence lists.
- Add `docs/schemas/rynix.contract.v1.json` describing required fields
  (`name`, `[[evidence]]` with `kind` ∈ {cargo_test, file}).
- Gate: `contract_schema_gate` validates all `docs/contracts/*.contract.toml`
  parse as having a `name` and ≥1 evidence row.

## Consequences

- Does not replace `rynixc verify`; it is a structural hygiene check.
