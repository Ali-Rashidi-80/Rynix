# ADR-0022: Build link sandbox (`--sandbox=docker|none`)

## Status

Accepted (Phase 27-A)

## Context

`rynixc build` invokes host `clang` on generated LLVM IR and `rt/` C sources.
[SECURITY.md](../../SECURITY.md) already warns that untrusted `.ryx` is like
untrusted C. Quality-10 wants an **opt-in** isolation path without making Docker
a hard dependency for everyday builds.

## Decision

- Add CLI flag `--sandbox=docker|none` on `rynixc build` (and documented for
  link-capable commands that share the same link path).
- **Default: `none`** — unchanged host clang link behavior.
- **`docker`**: run the clang link step inside `docker run` (network disabled
  when practical). Inputs are staged into a work directory bind-mounted into
  the container. Image defaults to `silkeh/clang:latest`, overridable via
  `RYNIX_DOCKER_IMAGE`.
- If `--sandbox=docker` is set and `docker` is missing or unusable: **hard
  error** with a clear message (no silent fallback to host link).
- CI hosts without Docker: skip the docker smoke and rely on
  [SANDBOX_SKIP_MATRIX.md](../SANDBOX_SKIP_MATRIX.md) (documented OK).
- Windows Job Object sandbox is **deferred** —
  [WINDOWS_SANDBOX_DEFERRAL.md](../WINDOWS_SANDBOX_DEFERRAL.md) (amend note).

## Consequences

- Gate: `sandbox_docker_smoke` (docker path *or* skip-matrix file).
- Does not sandbox the Rust frontend itself; only the clang link subprocess.
- Does not replace OS-level policy for untrusted source ingestion.
