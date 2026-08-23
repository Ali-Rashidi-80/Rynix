# ADR-0010: Local filesystem package index (no network registry)

Date: 2026-08-23  
Status: **Accepted**

## Context

Peers often market a “global package registry.” End’s registry surface is largely
staging/stub. Rynix already resolves **path** dependencies via `rynix.toml`
(`[dependencies] name = { path = "…" }`).

Agents still want versioned local reuse without inventing a fake CDN.

## Decision

1. **Do not** ship a network package registry for v0.1.
2. Support an optional **local filesystem index**:

```toml
[registry]
path = "vendor"

[dependencies]
util = "0.1.0"                 # → vendor/util/0.1.0/ or vendor/util-0.1.0/
lib  = { path = "../lib" }     # unchanged path form
```

3. Resolution supports **exact** version directories and **semver ranges**
   (`^1.2.3`, `>=1.2.3`, `=1.2.3`) against hierarchical
   `{registry}/{name}/{version}/` folders (highest match wins). Hyphenated
   `{name}-{version}` remains exact-only.
4. `rynixc deps` / build gate report `kind: "registry" | "path"` in `rynix.deps.v1`.
5. `rynixc build` / `emit-ll` **unity-compile** each dep `[package].entry` with
   `pkg__fn` mangling (SPEC §6.3–6.5). Soft builtins stay in sema; `import std::X`
   loads real `std/X.ryx` defs when present.

## Consequences

- SPEC §6 documents layouts, ranges, mangling, std loader, and honesty (no CDN).
- Tests: registry exact + caret (`pkg_semver_app`), mangled path/registry builds,
  `build_pkg_std_app_loads_math`.
- Global/registry CDN remains out of scope until a future ADR with mirror +
  checksum policy + CI evidence.
- Dependency: workspace `semver` (cold path only; ADR-0004).
