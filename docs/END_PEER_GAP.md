# Rynix vs End — honest peer gap analysis

Reference: [IrMaho/End](https://github.com/IrMaho/End) (v0.4.x positioning, 2026).

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

### Latest local Suite5 (2026-08-23, Windows) — Rynix rank / 6

| Workload | Best | Rynix | Rank | vs best |
|----------|------|------:|-----:|--------:|
| alu | rynix 5.7 | 5.7 | **1** | 0% |
| nested | end 5.4 | 5.9 | 2 | +10% |
| fib | rynix 5.6 | 5.6 | **1** | 0% |
| hash | rynix 5.1 | 5.1 | **1** | 0% |
| prime | rynix 8.3 | 8.3 | **1** | 0% |
| sum | rynix 5.8 | 5.8 | **1** | 0% |
| bits | rynix 89 | 89 | **1** | 0% |
| matrix | rynix 5.3 | 5.3 | **1** | 0% |
| scan | rynix 5.5 | 5.5 | **1** | 0% |
| powmod | rynix 11.8 | 11.8 | **1** | 0% |
| gcd | rynix 115 | 115 | **1** | 0% |
| reduce | rynix 7.9 | 7.9 | **1** | 0% |

Rynix leads **11/12**; End edges `nested` (~10%). Full ms matrix: root [README.md](../README.md).

Refresh: `python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary`

---

## 3. Is Rynix “more valuable” than End today?

**Overall: not yet.** End is a broader **product** (language + frameworks + editor
spectacle + agent contract system). Rynix is a narrower **verified systems compiler**
with stronger correctness gates on what it *does* ship.

### Where Rynix leads (evidence in-tree)

| Area | Evidence |
|------|----------|
| Checksum-gated microbench CI | `phase10_gates`, `suite5-check` job |
| LLVM ↔ interpreter differential | `diff_llvm_vs_interp` |
| Escape / alloc transparency | `--explain-alloc`, MCP explain |
| `@llvm.ctpop` / bits workload | Suite5 `bits` + RIR tests |
| Fiber + io_uring tests | `rt/tests/`, Linux CI |
| Honest docs / no fake ✅ | `AGENTS.md`, this document |

### Where End leads (from End README + tree; not independently verified here)

| Area | End claim / surface |
|------|---------------------|
| README & positioning | Badges, domain table, code examples, maturity matrix |
| Language surface | 4-tier memory, `operation` values, agent contracts |
| Frameworks | EndHyper, EndForge, EndNexus, EndCrypto, EndKV, UI canvas |
| Benchmark spectacle | suite12 heavy sims, small-binary marketing rows |
| Editor | CodeLens, sandbox webview |
| Backends | **C11 shipping** + LLVM alpha |
| Package story | `end.config.toml`, `end new`, registry (planned) |

### Overlap (both ship)

- AI CLI: graph / slice / impact / eval / patch / arch
- MCP / structured diagnostics
- VS Code + LSP (End richer in editor UX)
- Zero-GC / region-style memory narrative
- 12 × 5-lang benchmark *shape* (different workloads)

---

## 4. Gap backlog — priority for “lead without copy-paste”

Priority is **evidence-first** (test or CI before README ✅).

### P0 — benchmark fairness & End peer slot

- [x] Wire `end` in `run_suite5.py` when `endc`/`end` + `{ch}.end` exist
- [x] Port 12 Suite5 algorithms to `.end` (same algorithms as C)
- [x] Local run with End toolchain proving checksum parity on all 12
- [x] Harness copies `.end` to `target/suite5/` so End C11 emit cannot clobber `{name}.c`
- [x] Opaque trip counts on all Suite5 peers (including End)
- [ ] Optional CI job when `endc` is available on runners

### Adopted toolchain practices (not a clone of End)

| Practice | Rynix landing |
|----------|---------------|
| Aggressive clang LTO / strip for release builds | `build_cmd.rs` |
| Slim link for benches | `--bench` → `rt/bench_rt.c` (+ MSVCRT gcc link on Windows) |
| Selective loop metadata | `llvm.loop.unroll` / vectorize where proven |
| Fast `(x*k)%m` when `x < m` | RIR peephole (powmod) |
| Pattern strength reduction | closed forms, Stein gcd (`cttz`), matrix fib, hash poly, … |

### P1 — product surface End has, Rynix lacks

- [x] README “What Rynix is NOT”, domain table, maturity matrix
- [x] Binary size matrix (hello + Suite5 reduce) — see below
- [x] Agent contract approach as ADR ([0009](adr/0009-agent-contracts-toolchain.md)) — toolchain evidence, not End syntax clone
- [ ] C11 backend or documented alternative ([ADR-0008](adr/0008-deferred-c11-backend.md))

### Binary size (Windows gnu, local 2026-08-22)

| Binary | Size |
|--------|-----:|
| `examples/01_hello.ryx` → rynix (full RT) | **87.5 KiB** |
| Suite5 `reduce` C (`clang -O3`) | ~86 KiB |
| Suite5 `reduce` Rynix **`--bench`** + sink RT | **~18 KiB** (MSVCRT) |
| Suite5 `reduce` End (`endc --strip`) | ~45.5 KiB |
| Suite5 `reduce` Zig | ~192 KiB |
| Suite5 `reduce` Go | ~1636 KiB |

With `--bench`, Rynix Suite5 bins were **smaller than End strip** on that machine.
Full-runtime hello gate remains **&lt;300 KiB**.

### P2 — frameworks & domains

- [ ] HTTP server beyond `http_get_json_i64` smoke
- [ ] TLS, WebSocket, game/canvas ([ADR-0007](adr/0007-deferred-ui-frameworks.md))
- [ ] suite12-class workloads (optional `benchmarks/suite12/` with checksums)

### P3 — editor & release polish

- [ ] LSP CodeLens / richer VS Code
- [ ] Signed releases / GPG (Rynix ships SHA256SUMS today)

---

## 5. What we refuse to do

- Copy End README tables or claim End’s suite12 ms as Rynix scores.
- Mark ROADMAP ✅ without in-tree tests (`AGENTS.md`).
- Present Rust-only CI runs as “5-language proof”.
- Claim Suite5 proves identical instruction work across languages after strength reduction.

---

## 6. One-line verdict

**Rynix is more *auditable*; End is more *ambitious on product surface*.** To be
“better overall”, Rynix must grow **language + libraries + editor + benchmarks End
has**, while keeping the **checksum / diff / escape** bar.

See also: [COMPARE.md](COMPARE.md), [ROADMAP.md](ROADMAP.md),
[benchmarks/README.md](../benchmarks/README.md).
