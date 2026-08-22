# ADR-0005: Textual LLVM IR is the shipping backend

Status: accepted (2026-08-21); **complete for shipping**

## Context

The backend must produce LLVM IR for ThinLTO/`--gc-sections` size targets.
Binding LLVM in-process (`inkwell` / `llvm-sys`) requires a full LLVM
toolchain at *compiler build* time — heavy and fragile on Windows, and it
pins the compiler to one LLVM version.

## Decision

Rynix emits *textual* LLVM IR (`.ll`) from RIR and drives an external
`clang` for optimization, code generation, and linking
(`-O3 -flto=thin -ffunction-sections -Wl,--gc-sections`). Whole-program
reachability DCE happens earlier, at the RIR level.

This **is** the complete shipping backend. Differential tests
(`diff_llvm_vs_interp`) compare interpreter results to linked binary exit
codes. An in-process LLVM binding is not part of the product surface.

## Consequences

- The compiler builds with zero native LLVM dependencies.
- `.ll` output is diffable and testable.
- A `clang` installation is a runtime requirement for `rynixc build`.
