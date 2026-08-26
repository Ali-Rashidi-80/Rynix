# Windows Job Object sandbox — deferral

Phase 27-F companion to [ADR-0022](adr/0022-build-sandbox.md).

## Decision

**Deferred:** wrapping `clang` link under a Windows **Job Object** (memory /
process-tree limits) is not required for Quality-10 Phase 27.

## Why

- Opt-in Docker sandbox covers the isolation story where Docker exists.
- Job Object APIs are Windows-specific; portable CI already spans Linux +
  Windows host clang without Job Objects.
- Implementing correct handle inheritance + kill-on-job-close for clang+lld
  child processes needs dedicated soak time beyond this phase’s gates.

## Revisit when

- A Windows-only CI job wants process-tree caps without Docker Desktop, or
- ADR-0022 is amended to Accept Job Object with an in-tree smoke test.

Until then, gate `windows_sandbox_or_deferral` is satisfied by **this file**.
