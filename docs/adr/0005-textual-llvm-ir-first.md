# ADR-0005: Emit textual LLVM IR first; in-process LLVM later

Status: accepted (2026-08-21)

## Context

The backend (Phase 7) must produce LLVM IR to reach the <1MB binary target
via ThinLTO/LTO and `--gc-sections`. Binding LLVM in-process (`inkwell` /
`llvm-sys`) requires a full LLVM toolchain at build time — heavy and fragile
on Windows, and it pins the compiler to one LLVM version.

## Decision

Phase 7 step 1 emits *textual* LLVM IR (`.ll`) from RIR and drives an
external `clang` for optimization, code generation, and linking
(`-O3 -flto=thin -ffunction-sections -Wl,--gc-sections`). Whole-program
reachability DCE happens earlier, at the RIR level, so only functions
reachable from `main` are emitted at all.

Step 2 (after the language stabilizes) migrates to `inkwell` for in-process
codegen, custom pass pipelines, fat LTO, and later Polly/PGO experiments.

## Consequences

- The compiler builds with zero native dependencies through Phase 7 step 1;
  Windows development stays trivial.
- `.ll` output is diffable and testable with pattern-based golden tests.
- A `clang` installation becomes a runtime requirement for `rynixc build`
  (checked with a clear diagnostic), not a build-time one.
