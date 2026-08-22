# Rynix vs peers (honest)

This document is **product positioning**, not a scoreboard. For a detailed End gap
analysis and benchmark methodology, see **[END_PEER_GAP.md](END_PEER_GAP.md)**.

## vs End ([IrMaho/End](https://github.com/IrMaho/End))

Both target AI-first / Zero-GC / systems backends. Both compile with a **Rust**
driver (`endc` / `rynixc`) and lean on **C** for native runtime.

| Area | End (typical positioning) | Rynix (shipping today) |
|------|---------------------------|-------------------------|
| README / packaging | Strong (logo, badges, 12-bench matrix) | Suite5 **12** × 5 langs + bakeoff (checksum CI) |
| Benchmark matrix | **suite12** heavy sims (SDF, HFT, SHA, …) | **Suite5** integer microkernels (different algorithms) |
| End in same harness | End is a first-class suite12 lang | **`end` slot** in Suite5 when `endc` + `.end` ports exist |
| AI CLI | `graph` / `impact` / `slice` / `eval` / `patch` / `arch` | Same + MCP (11 tools) |
| Agent contracts | 50-feature Intent→Verify pipeline | Not shipped (gap — see END_PEER_GAP P1) |
| Editor | VS Code / LSP (CodeLens, studio) | VS Code + LSP (diag, hover, go-to-def) |
| Frameworks | EndHyper, EndForge, UI canvas | Deferred ([ADR-0007](adr/0007-deferred-ui-frameworks.md)) |
| Concurrency | Threads / channels / OpenMP (per End docs) | Colorless **fibers** + PARKED + fiber-aware **io_uring** |
| Memory story | 4-tier regions + leases + borrow | Escape → stack/region/heap + injected free |
| Backend | **C11 shipping** + LLVM alpha | Textual LLVM ([ADR-0005](adr/0005-textual-llvm-ir-first.md)); C11 deferred ([ADR-0008](adr/0008-deferred-c11-backend.md)) |
| Correctness gates | checksum per bench | **CI: C ↔ Rynix all 12** + LLVM↔interp diff |
| Release | Signed binaries (claimed) | GitHub Release + **SHA256SUMS** |

**Verdict (2026-08):** End leads on **editor richness**, **framework breadth**, and
**README spectacle**. Rynix leads on **test-gated correctness**, **escape transparency**,
and several **Suite5 microkernel rows vs C/Rust/Go/Zig** — but is **not** “better overall”
until language surface and End peer benchmarks catch up.

## Why the compiler is Rust (not Zig/Go)

- End’s compiler is also Rust.
- Zig/Go/C/Rust/**End** in benchmark folders are **peer implementations of workloads**,
  not the implementation language of the compiler.
- Rynix runtime is **C** (`rt/`).

Run all peers locally:

```sh
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary
python benchmarks/suite5/analyze_results.py
```

## What we refuse to copy blindly

- Marketing tables without reproducible harnesses.
- Marking features complete without in-tree tests.
- Comparing Suite5 ms to End suite12 ms as if they were the same programs.
- Inflating RPS or binary-size claims beyond gates in CI/tests.
