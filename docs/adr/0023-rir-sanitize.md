# ADR-0023: RIR sanitize — reject dangerous CallExt

## Status

Accepted (Phase 27-B)

## Context

RIR `CallExt` can name arbitrary external symbols that LLVM / the C runtime
might resolve (`system`, `exec*`, `popen`, `dlopen`, …). Soft builtins are a
closed allowlist; anything else that slips through as CallExt is an escape
hatch. Quality-10 wants a hard reject before emit.

## Decision

- Add `rynix_rir::sanitize_module`: scan every `CallExt` (and matching external
  names) against a denylist including at least:
  `system`, `exec`, `execl`, `execv`, `execve`, `execvp`, `popen`, `dlopen`,
  `rynix_rt_system`, and names with an `exec` prefix used as process spawn.
- Run sanitize in the codegen pipeline **before** LLVM emit (build / emit-ll /
  emit-wasm). Fail closed with a diagnostic mentioning `sanitize` and the name.
- Soft front-end: sema rejects free-function calls named `system` / `exec` /
  `popen` / `dlopen` (and close aliases) with a clear dangerous/reserved error
  (`RYX2014`), analogous to stub reserved names (`RYX2013`).

## Consequences

- Gate: `sanitize_rejects_exec`.
- Legitimate soft builtins stay allowlisted and are never denylisted.
- Does not claim full CFI or seccomp; this is a named-escape reject only.
