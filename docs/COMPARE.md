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
| AI CLI | `graph` / `impact` / `slice` / `eval` / `patch` / `arch` | Same + MCP (18 tools) + `verify`/`precheck`/`context`/`security`/`scope`/`deps`/`dna` |
| Agent contracts | 50-feature Intent→Verify pipeline | Wave 1 toolchain evidence ([ADR-0009](adr/0009-agent-contracts-toolchain.md)); not End syntax clone |
| Editor | VS Code / LSP (CodeLens, studio) | VS Code + LSP + CodeLens (check/alloc/impact) |
| Frameworks | EndHyper, EndForge, UI canvas | Deferred ([ADR-0007](adr/0007-deferred-ui-frameworks.md)) |
| Concurrency | Threads / channels / OpenMP (per End docs) | Colorless **fibers** + PARKED + io_uring (Linux) / IOCP (Windows) |
| Packages | Registry staging (per End docs) | Path deps + local filesystem index ([ADR-0010](adr/0010-local-package-index.md); no CDN) |
| Memory story | 4-tier regions + leases + borrow | Escape → stack/region/heap + injected free |
| Backend | **C11 shipping** + LLVM alpha | Textual LLVM ([ADR-0005](adr/0005-textual-llvm-ir-first.md)); C11 deferred ([ADR-0008](adr/0008-deferred-c11-backend.md)) |
| Correctness gates | checksum per bench | **CI: C ↔ Rynix all 12** + LLVM↔interp diff |
| Release | Signed binaries (claimed) | GitHub Release + **SHA256SUMS** + optional GPG (`release.yml`) |

**Verdict (2026-08-23):** End still leads on **framework/editor spectacle** and suite12
marketing rows. Rynix leads on **test-gated correctness**, **escape transparency**,
real HTTP/crypto/KV/TLS/WS, agent verify stack, local packages, Suite5 vs End when
strength reduction applies, and suite12 MATCH checksum ports where peers agree. See
[END_PEER_GAP.md](END_PEER_GAP.md) for methodology honesty.

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
