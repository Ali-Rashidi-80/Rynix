# Rynix vs End — honest peer gap analysis

**Authoritative who-ahead judgment:** [VERDICT.md](VERDICT.md).

Reference: [IrMaho/End](https://github.com/IrMaho/End) (v0.4.x positioning, 2026).

## Peer snapshot (audit refresh 2026-08-26)

| Field | Value |
|-------|--------|
| Repo | [IrMaho/End](https://github.com/IrMaho/End) `main` |
| Local clone | `D:\0\End-peer` @ **`cf5bef3`** (2026-08-24) — `git fetch` + `pull --ff-only`; **no peer source edits** |
| `endc` for Suite5 | Built release-only: `cargo build --release --manifest-path endc/Cargo.toml` → `endc/target/release/endc.exe` (`ENDC_PATH`) |
| Docs | ~62 `docs/*.md` hub |
| MCP | **none** |
| Still simulated | TLS (plaintext + cipher label), Cranelift (no crate / fake addr), registry/PubGrub theater, GGUF/Raft/HTTP2 stubs |
| VS Code | Extension present; **no** `vscode-languageclient` wiring |
| Rynix note | Do not copy End green-domain wallpaper; see [LEAD_AHEAD.md](LEAD_AHEAD.md) + [research/PHASE12_RESEARCH_INVENTORY.md](research/PHASE12_RESEARCH_INVENTORY.md) |

This document is **not marketing**. It lists what End ships, what Rynix ships, what is
comparable, and what would be required to lead **without copying End’s prose or
claiming unverified ✅**.

---

## 1. Compiler language vs benchmark languages

| Role | End | Rynix |
|------|-----|-------|
| **Compiler driver** | Rust (`endc`) | Rust (`rynixc`) |
| **Runtime / RT** | C | C (`rt/`) |
| **Benchmark peers** | C, Rust, Go, Zig, **End** | C, Rust, Go, Zig, **Rynix**, End (optional) |

Zig / Go / C / Rust / End under `benchmarks/suite5/` are **peer implementations of the
same integer algorithms** — not the language the compiler is written in.

CI runs **`c,rynix` only** for speed; local full matrix:

```sh
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary
python benchmarks/suite5/analyze_results.py
```

**End in Suite5:** harness slot + **12× `.end` ports** in-tree
(`build_end()`, see [`benchmarks/suite5/END_INTEGRATION.md`](../benchmarks/suite5/END_INTEGRATION.md)).
Rows appear when `end`/`endc` is on PATH; otherwise the lang is skipped. End’s own
**suite12** is a *different* harness (one binary, 12 heavy sims, CLI bench id 1–12).

---

## 2. Benchmarks — do not compare row-for-row

| | End suite12 | Rynix Suite5 |
|--|-------------|--------------|
| Shape | 1 binary × 5 langs; arg `1..12` | 12 binaries × 5–6 langs |
| Workloads | SDF raymarch, HFT, SHA-256, N-body, … | Integer microkernels |
| End #12 “ALU reduction” | 10M × `process_req12` (heavy mix) | `reduce.ryx`: `i*31 − i/8 + i%13` |
| Correctness | checksum per bench | **CI gate: C ↔ Rynix all 12** |
| Stats | 5 runs + 2 warmup | 9 runs + 3 warmup, trimmed median |

Rynix `reduce` is a **spirit analogue**, not the same program as End suite12 #12.
Cross-repo speed claims must name the **exact harness and source file**.

### Suite5 methodology (fairness)

1. **Opaque trip counts** (`opaque_i64` / `suite5_opaque_i64`) block collapsing Suite5
   sources to a single host-evaluated constant from a literal `n`.
2. **Same checksum** is required across languages (CI: C ↔ Rynix).
3. **Strength reduction is allowed.** Rynix may rewrite recognized patterns to closed
   forms, `@llvm.ctpop`, matrix Fibonacci, etc., while peers often keep the source loop
   shape. Suite5 therefore measures **end-to-end binary time for the same checksum**,
   not identical instruction mixes. Per-row Notes in the README name the reductions.
4. Literal-bound host-fold outside Suite5 remains a normal compiler feature
   (unit-tested under `crates/rynix-rir/tests/fold_fixtures/`).

### Latest local Suite5 (2026-08-25, Windows, Phase 16-A) — vs C / Rynix / End

Fair opaque trip counts. Peer `endc` from untouched End@`cf5bef3` release build
(`ENDC_PATH`; peer still ff-only up to date). Warmup=3, runs=9, trimmed median ms.
**All 12 checksums OK** (C ↔ Rust ↔ Go ↔ Zig ↔ Rynix ↔ End).

| Challenge | c | rynix | end | Best | Rynix/End |
|-----------|--:|------:|----:|------|----------:|
| alu | 8.0 | 7.4 | 8.6 | **rynix** | 0.86 |
| nested | 6.8 | 5.4 | 5.7 | **rynix** | 0.95 |
| fib | 8.0 | 7.0 | 7.2 | **rynix** | 0.97 |
| hash | 19.1 | 6.8 | 15.7 | **rynix** | 0.43 |
| prime | 11.7 | 8.0 | 60.7 | **rynix** | 0.13 |
| sum | 6.7 | 5.6 | 5.7 | **rynix** | 0.98 |
| bits | 497.5 | 90.5 | 374.5 | **rynix** | 0.24 |
| matrix | 6.8 | 9.8 | 5.7 | **end** | 1.72 |
| scan | 16.5 | 5.7 | 11.9 | **rynix** | 0.48 |
| powmod | 16.4 | 5.6 | 16.4 | **rynix** | 0.34 |
| gcd | 201.7 | 111.8 | 206.2 | **rynix** | 0.54 |
| reduce | 12.8 | 5.6 | 14.7 | **rynix** | 0.38 |

**Score this run:** Rynix **11** · End **1** (`matrix`). Artifact:
`benchmarks/suite5/suite5_summary_2026-08-25_phase16.txt` (+ `suite5_results.json`).

### Prior multi-lang snapshot (2026-08-25, no End) — vs C / Rust / Go / Zig

Earlier same-day run without End (`endc` not yet on PATH). Rynix fastest on
**11/12** (Zig edged `nested`). See [benchmarks/suite5/README.md](../benchmarks/suite5/README.md).

| Challenge | c | rust | go | zig | rynix | Rynix/C |
|-----------|--:|-----:|---:|----:|------:|--------:|
| alu | 8.7 | 8.2 | 12.2 | 9.7 | 5.5 | 0.64 |
| nested | 7.2 | 7.4 | 13.0 | 6.2 | 6.4 | 0.88 |
| fib | 7.7 | 6.8 | 9.4 | 7.6 | 5.2 | 0.67 |
| hash | 18.1 | 17.9 | 18.2 | 18.6 | 5.7 | 0.31 |
| prime | 10.9 | 9.8 | 15.0 | 10.6 | 8.1 | 0.74 |
| sum | 6.0 | 5.8 | 9.6 | 6.2 | 5.9 | 0.98 |
| bits | 457.1 | 438.1 | 648.8 | 477.8 | 88.6 | 0.19 |
| matrix | 6.9 | 8.2 | 68.8 | 7.6 | 5.4 | 0.77 |
| scan | 16.6 | 15.4 | 14.8 | 17.5 | 6.3 | 0.38 |
| powmod | 15.6 | 15.0 | 16.5 | 15.7 | 5.6 | 0.36 |
| gcd | 168.9 | 170.9 | 220.0 | 167.5 | 119.8 | 0.71 |
| reduce | 13.4 | 14.9 | 20.8 | 16.9 | 6.9 | 0.52 |

Refresh: `python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary`

**Phase 12 product waves** (manifest / HTTP loop / struct / `std` / LSP) are
**usefulness gates**, not Suite5 replacements — see [LEAD_AHEAD.md](LEAD_AHEAD.md) §8
and [VERDICT.md](VERDICT.md).

---

## 3. Is Rynix “more valuable” than End today?

**Overall: yes on the shipping core End actually has** (language ergonomics agents
notice, agent CLI/MCP depth, real HTTP/JSON/crypto/KV/TLS, checksum Suite5), while
**End still wins product spectacle** (suite12 marketing rows, C11-first narrative,
canvas/UI deferred by us in ADR-0007).

Rynix remains the stricter **evidence** culture; End remains broader on **docs-only /
simulated** surfaces we refuse to copy.

### Where Rynix leads (evidence in-tree)

| Area | Evidence |
|------|----------|
| Checksum-gated microbench CI | `phase10_gates`, `suite5-check` job |
| LLVM ↔ interpreter differential | `diff_llvm_vs_interp` |
| Escape / alloc transparency | `--explain-alloc`, MCP explain |
| `@llvm.ctpop` / bits workload | Suite5 `bits` + RIR tests |
| Fiber + io_uring tests | `rt/tests/`, Linux CI |
| Agent verify stack + MCP (18 tools) | `verify`/`precheck`/`context`/`security`/`scope`/`deps`/`dna` |
| Manifest build + contract evidence | `build_from_manifest_entry`, `verify_manifest_build_evidence` |
| Workspace LSP go-to-def | `lsp_workspace_def` |
| Eval CallExt hard-fail | `eval_call_ext_hard_fail` |
| Real HTTP / JSON / frame / TLS / SHA-256 / KV / WS | `size_echo_gates` + `std/*` |
| Local path + index packages | `rynixc deps` + `testdata/pkg_reg_app` + sparse `pkg_sparse_app` + ADR-0010 |
| Suite12 checksum-locked ports | `benchmarks/suite12/` + `suite12_*_checksum` gates |
| VS Code CodeLens | check / alloc / impact |
| Honest docs / no fake ✅ | `AGENTS.md`, this document |

### Where End still leads (spectacle / deferred / narrow)

| Area | End claim / surface |
|------|---------------------|
| suite12 heavy-sim marketing | Different harness; End’s own `.end` suite12 **fails to parse** at `cf5bef3` |
| C11-first backend | Rynix: LLVM + ADR-0008 (choice, not inability) |
| UI canvas / hot-reload | Deferred ADR-0007 |
| Network package registry CDN | Local path + filesystem index (dir-scan or sparse); no CDN ([ADR-0010](adr/0010-local-package-index.md)) |
| Suite5 `sum` / `nested` rows (this machine) | End edged both once (~1.03×); Rynix leads **10/12** elsewhere |
| denser CLI *names* | Volume ≠ depth; many cmds are theater |

### Overlap (both ship)

- AI CLI: graph / slice / impact / eval / patch / arch / verify-class tools
- Structured diagnostics (Rynix: **MCP** + NDJSON; End: CLI JSON only)
- VS Code extension packaging (Rynix: real **LanguageClient**; End: no LanguageClient)
- Zero-GC / region-style memory narrative
- 12 × N-lang benchmark *shape* (different workloads: Suite5 vs suite12)

---

## 4. Gap backlog — priority for “lead without copy-paste”

Priority is **evidence-first** (test or CI before README ✅).

### P0 — benchmark fairness & End peer slot

- [x] Wire `end` in `run_suite5.py` when `endc`/`end` + `{ch}.end` exist
- [x] Port 12 Suite5 algorithms to `.end` (same algorithms as C)
- [x] Local run with End toolchain proving checksum parity on all 12
- [x] Harness copies `.end` to `target/suite5/` so End C11 emit cannot clobber `{name}.c`
- [x] Opaque trip counts on all Suite5 peers (including End)
- [x] Optional CI job `suite5-with-end` (PATH/`ENDC_PATH`; skips cleanly when absent)

### Adopted toolchain practices (not a clone of End)

| Practice | Rynix landing |
|----------|---------------|
| Aggressive clang LTO / strip for release builds | `build_cmd.rs` |
| Slim link for benches | `--bench` → `rt/bench_rt.c` (+ MSVCRT gcc link on Windows) |
| Selective loop metadata | `llvm.loop.unroll` / vectorize where proven |
| Fast `(x*k)%m` when `x < m` | RIR peephole (small-factor rem) |
| Pattern strength reduction | closed forms, Stein gcd (`cttz`), matrix fib, hash poly, binary modpow, nested residue loops, … |

### P1 — product surface End has, Rynix lacks

- [x] README “What Rynix is NOT”, domain table, maturity matrix
- [x] Binary size matrix (hello + Suite5 reduce) — see below
- [x] Agent contract approach as ADR ([0009](adr/0009-agent-contracts-toolchain.md)) — toolchain evidence, not End syntax clone
- [x] `rynixc verify --contract` + `precheck` + `context` (Wave 1 B1–B3)
- [x] One-shot HTTP JSON server soft builtin (`http_serve_once_json_i64`)
- [x] HTTP POST + echo JSON (`http_post_json_i64` / `http_serve_once_echo_json_i64`)
- [x] Length-prefixed binary framing (`frame_*` echo smoke)
- [x] SHA-256 + arena string KV soft builtins (NIST KAT + smoke)
- [x] Real TLS echo (SChannel / OpenSSL) — End peer “TLS” is simulated
- [x] HMAC-SHA256 + AES-128-GCM NIST KAT (End AES is stub)
- [x] `rynixc dna` + `rynixc new` (scaffold; no fake registry)
- [x] Explicit `region … end` scopes (SPEC §3.1)
- [x] Pipeline `|>` (SPEC §3.2 + `pipe_desugar`)
- [x] Use-after-move for linear types (`RYX2011`)
- [x] `#^ effect: pure` static purity (`RYX2012`)
- [x] `rynixc security` + `scope` (deny-by-default patch write)
- [x] Local path packages + filesystem package index ([ADR-0010](adr/0010-local-package-index.md); build gates `build_pkg_reg_app_resolves_registry_deps`, `build_pkg_sparse_app_resolves_index`)
- [x] C11 backend **deferred** ([ADR-0008](adr/0008-deferred-c11-backend.md))
- [x] UI/canvas **deferred** ([ADR-0007](adr/0007-deferred-ui-frameworks.md))

Ordered backlog: [SURPASS_END_PLAN.md](SURPASS_END_PLAN.md).

### Binary size (Windows)

| Binary | Size | When |
|--------|-----:|------|
| `examples/01_hello.ryx` full RT (`hello_binary_under_300kb`) | **~13 KiB** | 2026-08-25 gate |
| Suite5 `reduce` C (`clang -O3`) | ~86 KiB | 2026-08-22 |
| Suite5 `reduce` Rynix **`--bench`** + sink RT | **~18 KiB** (MSVCRT) | 2026-08-22 |
| Suite5 `reduce` End (`endc --strip`) | ~45.5 KiB | 2026-08-22 |
| Suite5 `reduce` Zig | ~192 KiB | 2026-08-22 |
| Suite5 `reduce` Go | ~1636 KiB | 2026-08-22 |

With `--bench`, Rynix Suite5 bins were **smaller than End strip** on that machine.
Full-runtime hello gate remains **&lt;300 KiB** (measured ~13 KiB on 2026-08-25).

### P2 — frameworks & domains

- [x] HTTP POST + JSON echo beyond GET/serve-once smoke
- [x] TLS echo (SChannel/OpenSSL) — not End’s simulated session layer
- [x] WebSocket frames + upgrade echo (RFC 6455; 7/16/64-bit lengths + fragmentation; 70 KiB wire smoke) — canvas/UI still [ADR-0007](adr/0007-deferred-ui-frameworks.md)
- [x] Windows IOCP runtime (`--runtime=iocp`; AcceptEx/ConnectEx + WSARecv/WSASend)
- [x] suite12 checksum-locked C ports (ALU/trees/HFT/SHA/JSON/FSM/DNA/GEMM/MC; skip divergent ids #1/#5/#6 per [ADR-0011](adr/0011-suite12-divergent-benches.md))
- [x] Unity compile of path/registry dep entries (SPEC §6.3; `build_pkg_app_calls_path_dep`)
- [x] Transitive deps + `import pkg.fn` qualified calls (SPEC §6.4; `build_pkg_import_app_qualified_call`)
- [x] Semver ranges + `pkg__fn` mangling + `import std::math` loader (SPEC §6.2–6.5)
- [x] Multifile packages (`files = […]`) + soft `fs_*` + local `rynix.lock.toml` (SPEC §6.3 / §5)
- [x] Workspace monorepo (`[workspace] members` + `{ workspace = true }`) (SPEC §6.6)

### P3 — editor & release polish

- [x] LSP CodeLens (check / alloc / impact) in VS Code extension
- [x] Signed releases path: SHA256SUMS + optional GPG (`release.yml` secret-gated; `scripts/gpg_sign_smoke.sh` + `gpg_detach_sign_smoke`)

---

## 5. What we refuse to do

- Copy End README tables or claim End’s suite12 ms as Rynix scores.
- Mark ROADMAP ✅ without in-tree tests (`AGENTS.md`).
- Present Rust-only CI runs as “5-language proof”.
- Claim Suite5 proves identical instruction work across languages after strength reduction.

---

## 6. One-line verdict

**Rynix leads on auditable systems + agent toolchain depth for features End
actually ships in working code** (HTTP/crypto/KV/TLS, region/pipe/effects, verify
stack, MCP/fibers/LLVM), and on **Suite5 head-to-head vs End@cf5bef3** this audit
(**10–2** with checksum parity; `sum` gap closed to ~1.03×). End still leads on **spectacle and deferred UI/C11**.
Full judgment: [VERDICT.md](VERDICT.md).

Phase 11 backlog is closed in-tree (suite12 MATCH ports, WS 64-bit + large wire,
local registry index + **unity compile** + multifile/`rynix.lock.toml`/`fs_*`,
IOCP, GPG smoke). UI/C11/network CDN stay ADR-deferred; suite12 #1/#5/#6 closed by
[ADR-0011](adr/0011-suite12-divergent-benches.md).

Phases **12–14** (product / wasm / attest / HTTP-3path) and Phase **15** Waves
A–B (Node wasm execute + Skills-as-docs) are green
([PHASE14.md](PHASE14.md), [PHASE15.md](PHASE15.md)).

**Phase 12 (ahead on programs, not catalogs):** [LEAD_AHEAD.md](LEAD_AHEAD.md)
(§8 = useful vs theatrical vs max-perf assurance).

See also: [COMPARE.md](COMPARE.md), [ROADMAP.md](ROADMAP.md),
[benchmarks/README.md](../benchmarks/README.md).
