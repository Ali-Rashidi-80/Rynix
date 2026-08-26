# Docker sandbox skip matrix

Gate companion for Phase 27-A (`sandbox_docker_smoke`) and
[ADR-0022](adr/0022-build-sandbox.md).

`--sandbox=docker` is **opt-in**. Default remains `--sandbox=none` (host clang).

## When docker sandbox is skipped / not required

| Host / CI | Docker available? | Expected behavior |
|-----------|-------------------|-------------------|
| Local Windows / Linux without Docker Desktop / engine | No | `--sandbox=docker` **errors** clearly; everyday `build` uses `none` |
| GitHub Actions Ubuntu **without** Docker service | No | Skip docker smoke; this matrix documents OK for CI |
| GitHub Actions / local **with** Docker **and** local clang image | Yes | Full `build --sandbox=docker` smoke |
| Docker up but image not local / no registry | Partial | Skip full link; matrix documents OK (avoid pull hangs) |
| Agents / constrained sandboxes | Often no | Treat like “no docker”; do not fail the phase on skip |

## Test contract

`sandbox_docker_smoke`:

1. Asserts this file exists and mentions `docker`.
2. If `docker info` fails → **pass** (skip path).
3. If docker works → attempt a tiny fixture build with `--sandbox=docker`.

## Notes

- Missing docker under `--sandbox=docker` is a **hard error**, not a silent
  host fallback (ADR-0022).
- Image: `RYNIX_DOCKER_IMAGE` or default `silkeh/clang:latest`.
