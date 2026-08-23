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

3. Resolution is exact-version only (no semver ranges yet).
4. `rynixc deps` / build gate report `kind: "registry" | "path"` in `rynix.deps.v1`.
5. `rynixc build` / `emit-ll` **unity-compile** each dep `[package].entry` with the
   app (flat symbols; SPEC §6.3). Resolve-only is insufficient.

## Consequences

- SPEC §6 documents the layouts, unity compile, and honesty bound (no network).
- Tests: `manifest::resolves_local_registry_layout`, `deps_resolves_local_registry_version`,
  `build_pkg_app_calls_path_dep`, `build_pkg_reg_app_resolves_registry_deps`.
- Global/registry CDN remains out of scope until a future ADR with mirror +
  checksum policy + CI evidence. **Wave close (2026-08-23):** no stub CDN —
  local index + unity compile is the shipping package story.
