# Rynix security threat model (STRIDE)

Short, honest model for the **local compiler + runtime** surface. Complements
[SECURITY.md](../SECURITY.md). Does **not** claim Absolute-10 or “untrusted
`.ryx` is safe.”

## Assets

| Asset | Notes |
|-------|--------|
| Host filesystem / process | `build`/`run` invoke clang + linked `rt/` |
| Source secrets in tree | Pattern scan (`rynixc security`) is advisory |
| MCP / LSP client trust | Client can supply source; run only with trusted clients |
| Generated IR / binaries | Treated like compiling C |

## STRIDE summary

| Category | Threat | Mitigations (current) | Residual |
|----------|--------|------------------------|----------|
| **S**poofing | Impersonating package / registry | Path + local index only; no network registry | Local index still trust-the-admin |
| **T**ampering | Malicious `.ryx` / deps altering build | Scope gate for `patch --write`; attest digests local-only | No Sigstore Rekor |
| **R**epudiation | Unclear who built what | Local `rynix.attest.v1` digests | Not a chain of custody |
| **I**nformation disclosure | Secrets in source | CWE-798-class pattern scan | Heuristic; not taint |
| **D**enial of service | Pathological source / clang hang | Fuzz seeds; CI timeouts | No hard compile budget |
| **E**levation of privilege | `system`/`exec*`/`dlopen` escapes via CallExt | RIR sanitize + sema reject ([ADR-0023](adr/0023-rir-sanitize.md)); opt-in docker link ([ADR-0022](adr/0022-build-sandbox.md)) | Host link default (`--sandbox=none`); no seccomp required |

## Explicit non-goals (Phase 27)

- Guaranteeing safety of arbitrary untrusted programs
- Full CWE coverage or formal proofs
- Windows Job Object (see [WINDOWS_SANDBOX_DEFERRAL.md](WINDOWS_SANDBOX_DEFERRAL.md))
