# Deferred: C11 shipping backend

Date: 2026-08-22  
Status: **Deferred** — LLVM textual IR is the shipping backend ([ADR-0005](0005-textual-llvm-ir-first.md))

## Context

Some peers ship a C11 code generator for portability and tiny-toolchain deploys.
Rynix already links a **C runtime** (`rt/portable.c`) with LLVM-generated object
code via `clang`.

## Decision

Do **not** implement a Rynix→C11 transpiler for v0.1. Keep:

- Shipping: textual LLVM IR + ThinLTO (`rynixc emit-ll` / `build`)
- Runtime: C unity build (`rt/portable.c`)

Revisit only if ADR-0005 is superseded with differential tests LLVM↔C11.

## Consequences

- ROADMAP “Optional C11 backend” stays open with this ADR as the gate.
- `docs/COMPARE.md` documents LLVM-first honestly vs C11-first peers.
