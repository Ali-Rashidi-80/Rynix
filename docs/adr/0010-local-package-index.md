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
4. Optional **local sparse index** (Cargo crate-file layout, **no HTTP**):
   `{registry}/index/config.json` plus NDJSON crate files at
   `index/{prefix}/{name}` (or `.json`). Auto-detected; `[registry] sparse = true`
   requires the config file; `sparse = false` forces directory scan.
   Listed `vers` (skipping `yanked: true`) are authoritative — extra on-disk
   version dirs that are not listed must not win. Package sources still live
   under `{registry}/{name}/{version}/`.
5. `rynixc deps` / build gate report `kind: "registry" | "path"` and
   `registry_index: "sparse" | "scan"` in `rynix.deps.v1`.
6. `rynixc build` / `emit-ll` **unity-compile** each dep’s `entry` plus optional
   `[package].files` with `pkg__fn` mangling (SPEC §6.3–6.5). Soft builtins stay
   in sema; `import std::X` loads real `std/X.ryx` defs when present.
7. Optional **`rynix.lock.toml`** pins resolved local deps with sha256 over
   ordered sources (`rynixc deps --lock` / `--locked`; verify on build when
   present). Lock path is the workspace root when applicable. Still no network CDN.

## Consequences

- SPEC §6 documents layouts, ranges, mangling, multifile `files`, local lock,
  std loader, and honesty (no CDN).
- Tests: registry exact + caret (`pkg_semver_app`), sparse index
  (`pkg_sparse_app`, `deps_resolves_sparse_local_index`), mangled path/registry
  builds, multifile `pkg_util` extras, `build_pkg_std_app_loads_math`,
  `deps_lock_write_verify_and_tamper`.
- Global/registry CDN remains out of scope until a future ADR with mirror +
  checksum policy + CI evidence.
- Dependencies: workspace `semver` + `sha2` (cold path only; ADR-0004).
