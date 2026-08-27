# Windows Job Object sandbox — Implemented (Phase 31)

**Status:** Superseded. `--sandbox=job` wraps the clang link child in a Windows
Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, 1 GiB process memory, max 32
processes). Hard-errors on non-Windows.

Gate: `windows_sandbox_smoke`. See [ADR-0022](adr/0022-build-sandbox.md).
