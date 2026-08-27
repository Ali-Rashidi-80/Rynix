# ADR-0022: Build link sandbox (`--sandbox=docker|none|job`)

## Status

Accepted (Phase 27-A); **amended Phase 31** — Windows Job Object Implemented.

## Context

`rynixc build` invokes host `clang` on generated LLVM IR and `rt/` C sources.
[SECURITY.md](../../SECURITY.md) already warns that untrusted `.ryx` is like
untrusted C. Quality-10 wants an **opt-in** isolation path without making Docker
a hard dependency for everyday builds.

## Decision

- CLI flag `--sandbox=none|docker|job` on `rynixc build`.
- **Default: `none`** — unchanged host clang link behavior.
- **`docker`**: run the clang link step inside `docker run` (network disabled
  when practical). Hard-error if docker missing.
- **`job` (Windows):** assign clang child to a Job Object with kill-on-close,
  process memory cap (1 GiB), active process limit (32). Hard-error on
  non-Windows.
- CI hosts without Docker: skip docker smoke via
  [SANDBOX_SKIP_MATRIX.md](../SANDBOX_SKIP_MATRIX.md).

## Consequences

- Gates: `sandbox_docker_smoke`, `windows_sandbox_smoke`.
- Does not sandbox the Rust frontend; does not claim untrusted `.ryx` is safe.
