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
| **Benchmark peers** | C, Rust, Go, Zig, **End** | C, Rust, Go, Zig, **Rynix** |

**Zig / Go / C / Rust in `benchmarks/suite5/` are peer implementations of the same
integer algorithms** — not the language the compiler is written in. Both projects
use the same industry pattern.

CI runs **`c,rynix` only** for speed; local full matrix:

```sh
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix --summary
python benchmarks/suite5/analyze_results.py   # rank & gap-to-fastest
```

**End language in Suite5:** harness slot + **12× `.end` ports** are in-tree
(`build_end()`, see `benchmarks/suite5/END_INTEGRATION.md`). Rows appear when
`end`/`endc` is on PATH; otherwise the lang is skipped. End’s own **suite12** is
a *different* harness (one binary, 12 heavy sims, CLI bench id 1–12).

---

## 2. Benchmarks — do not compare row-for-row

| | End suite12 | Rynix Suite5 |
|--|-------------|--------------|
| Shape | 1 binary × 5 langs; arg `1..12` | 12 binaries × 5 langs |
| Workloads | SDF raymarch, HFT, SHA-256, N-body, … | Integer microkernels |
| End #12 “ALU reduction” | 10M × `process_req12` (50× hash mix) ≈ **650 ms** | `reduce.ryx`: `i*31-i/8+i%13` ≈ **11 ms** |
| Correctness | checksum per bench | **CI gate: C ↔ Rynix all 12** |
| Stats | 5 runs + 2 warmup | 9 runs + 3 warmup, trimmed median |

Rynix `reduce` is a **spirit analogue**, not the same program as End suite12 #12.
Cross-repo speed claims must name the **exact harness and source file**.

### Latest local Suite5 (2026-08-22, Windows) — Rynix rank / 5

| Workload | Best | Rynix | Rank | vs best |
|----------|------|-------|------|---------|
| alu | rynix 7.1 | 7.1 | **1** | 0% |
| nested | zig 6.3 | 6.5 | 3 | +3% |
| fib | c 7.5 | 7.3 | **1** | −3% |
| hash | rynix 16.3 | 16.3 | **1** | 0% |
| prime | rust 10.9 | 11.2 | 3 | +3% |
| sum | zig 5.9 | 6.3 | 3 | +7% |
| bits | rynix 91.5 | 91.5 | **1** | −79% vs C |
| matrix | rynix 6.8 | 6.8 | **1** | 0% |
| scan | c 8.5 | 8.7 | 2 | +2% |
| powmod | rust 14.3 | 15.5 | 4 | +8% |
| gcd | rynix 159.7 | 159.7 | **1** | 0% |
| reduce | c 10.5 | 11.2 | 2 | +7% |

Rynix leads **6/12** on this run; parity (~±10%) on most others. **Go** is slow on
`matrix` (65 ms) — implementation/port issue, not Rynix.

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
| `@llvm.ctpop` / bits workload | `popcount_ctpop.rs`, Suite5 `bits` |
| Fiber + io_uring tests | `rt/tests/`, Linux CI |
| Honest README / no fake ✅ | `AGENTS.md`, this doc |

### Where End leads (from End README + tree; not independently verified here)

| Area | End claim / surface |
|------|---------------------|
| README & positioning | Badges, domain table, code examples, maturity matrix |
| Language surface | 4-tier memory, `operation` values, agent contracts (50 features) |
| Frameworks | EndHyper, EndForge, EndNexus, EndCrypto, EndKV, UI canvas |
| Benchmark spectacle | suite12 heavy sims, 40 KB binary row |
| Editor | CodeLens, 120 FPS sandbox webview |
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
- [x] Port 12 Suite5 algorithms to `.end` (same algorithms as C; validate checksums with End installed)
- [ ] Local/CI run with End toolchain proving checksum parity
- [ ] Document suite12 vs Suite5 side-by-side (optional wrapper; no ms conflation)

### P1 — product surface End has, Rynix lacks

- [ ] README parity: “What Rynix is NOT”, domain table, maturity matrix (honest statuses)
- [ ] Binary size matrix (hello + one Suite5 bin) with reproduced flags
- [ ] Agent contract / skill / task syntax (Rynix-native design — not End clone)
- [ ] C11 backend or documented alternative ([ADR-0008](adr/0008-deferred-c11-backend.md))

### P2 — frameworks & domains (End’s “every domain” table)

- [ ] HTTP server beyond `http_get_json_i64` smoke
- [ ] TLS, WebSocket, game/canvas ([ADR-0007](adr/0007-deferred-ui-frameworks.md))
- [ ] suite12-class workloads (optional `benchmarks/suite12/` ports with checksums)

### P3 — editor & release polish

- [ ] LSP CodeLens / richer VS Code (End-level)
- [ ] Signed releases / GPG (End claims; Rynix has SHA256SUMS)

---

## 5. What we refuse to do

- Copy End README tables or claim End’s suite12 ms as Rynix scores.
- Mark ROADMAP ✅ without in-tree tests (`AGENTS.md`).
- Add “incremental mod”-style opts that block LLVM vectorization without benchmark proof.
- Present Rust-only CI runs as “5-language proof”.

---

## 6. One-line verdict

**Rynix is more *auditable*; End is more *ambitious on product surface*.** To be
“better overall”, Rynix must grow **language + libraries + editor + benchmarks End
has**, while keeping the **checksum / diff / escape** bar End does not emphasize equally.

See also: [COMPARE.md](COMPARE.md), [ROADMAP.md](ROADMAP.md), [benchmarks/README.md](../benchmarks/README.md).
