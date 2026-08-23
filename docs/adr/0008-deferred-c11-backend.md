# Deferred: C11 shipping backend

Date: 2026-08-22  
Reaffirmed: 2026-08-23  
Status: **Deferred** — LLVM textual IR is the shipping backend ([ADR-0005](0005-textual-llvm-ir-first.md))

## Context

Some peers ship a C11 code generator for portability and tiny-toolchain deploys.
Rynix already links a **C runtime** (`rt/portable.c`) with LLVM-generated object
code via `clang`. That is a C ABI + CRT story, **not** a Rynix→C11 transpiler.

## Decision

Do **not** implement a Rynix→C11 transpiler for v0.1. Keep:

- Shipping: textual LLVM IR + ThinLTO (`rynixc emit-ll` / `build`)
- Runtime: C unity build (`rt/portable.c`)

**Documented alternative (accepted for competitive honesty):** LLVM IR + C runtime
satisfies “ships native binaries via C toolchain” without duplicating End’s
C11-emit path. Size gates (`--bench`, hello &lt;300 KiB) are the evidence surface.

Revisit only when **all** of the following hold:

1. ADR-0005 is superseded (or amended) with an explicit dual-backend plan
2. Differential tests LLVM↔C11 exist for a fixed corpus (checksum or golden `.c`)
3. A real portability gap appears that clang+LLVM cannot cover

Until then, stubs that print C without semantics are **forbidden** (AGENTS.md).

## Consequences

- ROADMAP “Optional C11 backend” stays 🔄 deferred with this ADR as the gate
- `docs/COMPARE.md` / `END_PEER_GAP.md` may mark “documented alternative” ✅
  for the *decision*, never for a fake transpiler
- SURPASS D5 is closed by this ADR reaffirmation, not by shipping C11 emit
- Competitive “beyond Surpass” wave (2026-08-23): **no C11 transpile** — LLVM + C
  runtime remains the honest alternative; revisit criteria above are unchanged
- Follow-on wave: local package **index** (ADR-0010) is not a C11 backend
