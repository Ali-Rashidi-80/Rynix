# Rynix vs peers (honest)

This document is a **product positioning** note, not a scoreboard.

## vs End ([IrMaho/End](https://github.com/IrMaho/End))

Both target AI-first / Zero-GC / systems backends. Both compile with a **Rust**
driver (`endc` / `rynixc`) and lean on **C** for native runtime or C11 output.

| Area | End (typical positioning) | Rynix (shipping today) |
|------|---------------------------|-------------------------|
| README / packaging | Strong (logo, badges, 12-bench matrix) | Suite5 **12** × 5 langs + bakeoff (checksum CI) |
| Benchmark matrix | Large suite12 vs C/Zig/Rust/Go/End | Suite5 **12** micro + TCP bakeoff |
| AI CLI | `graph` / `impact` / `slice` / `eval` / `patch` / `arch` | Same + MCP (11 tools) |
| Editor | VS Code / LSP (CodeLens, studio) | VS Code + LSP (diag, hover, go-to-def) |
| Hot-reload / UI canvas / frameworks | Present in End docs | Deferred ([ADR-0007](adr/0007-deferred-ui-frameworks.md)) |
| Concurrency | Threads / channels / OpenMP (per End docs) | Colorless **fibers** + PARKED + fiber-aware **io_uring** |
| Memory story | Explicit `region` + tiers | Escape → stack/region/heap + injected free |
| Backend | C11 (+ LLVM/Cranelift in tree) | Textual LLVM ([ADR-0005](adr/0005-textual-llvm-ir-first.md)); C11 deferred ([ADR-0008](adr/0008-deferred-c11-backend.md)) |
| Collections | Broader std claims | Mono `Vec[i64]`/`Map[i64,i64]` ([ADR-0006](adr/0006-monomorphized-collections.md)) |
| Release | Signed binaries (claimed) | GitHub Release + **SHA256SUMS** (GPG optional) |

**Verdict:** End still leads on **editor richness** (CodeLens, studio, frameworks)
and **12-bench spectacle**. Rynix **matches End's AI CLI + arch check** while
leading on **test-gated correctness** (LLVM↔interp, escape explain, ASan runtime,
honest Suite5 checksums).

## Why the compiler is Rust (not Zig/Go)

- End’s compiler is also Rust.
- Zig/Go/C in benchmark folders are **peer implementations of workloads**, not
  the language implementation language.
- Rynix runtime is already **C** (`rt/`).

## What we refuse to copy blindly

- Marketing tables without reproducible harnesses.
- Marking features complete without in-tree tests.
- Inflating RPS or binary-size claims beyond gates in CI/tests.
