# Suite12 vs Rynix Suite5 — honesty note (SURPASS E2)

End ships `benchmarks/suite12/` (one binary × benches `1..12`, heavy sims:
raymarch, HFT, N-body, …). Stored End results sometimes **diverge in checksum**
from C/Rust/Go/Zig peers — so End’s “same algorithm” marketing is not an
evidence gate.

## What Rynix does instead

| Harness | Role |
|---------|------|
| [`../suite5/`](../suite5/) | **Gated** 12× integer kernels; CI requires C ↔ Rynix checksum match |
| This folder | **Checksum-locked C ports** of End suite12 algos that *agree* across peers |

## Policy (do not weaken)

1. Do **not** publish cross-repo millisecond tables against End suite12 unless
   every language prints the **same** `checksum=` for that bench id.
2. Prefer porting a bench into Suite5 (opaque trip counts + Notes) over cloning
   End’s single-binary spectacle.
3. Spirit analogues (e.g. Suite5 `reduce` vs End #12 ALU mix) must be labeled
   as such in [docs/END_PEER_GAP.md](../../docs/END_PEER_GAP.md).
4. Skip ports where End/C/peers diverge (raymarcher #1, n-body #5, ring #6).

## Locked ports (CI)

| File | End id | Locked `checksum=` | Notes |
|------|--------|--------------------|-------|
| `alu_reduction.c` | #12 | `3370198876750320971` | MATCH all langs incl. End |
| `hft_engine.c` | #3 | `552829538` | MATCH all langs |
| `json_serializer.c` | #8 | `5588438541400559045` | MATCH all langs |
| `fsm_lexer.c` | #9 | `-103069600432064540` | MATCH C/Zig/Rust/Go; End historically diverges |
| `dna_levenshtein.c` | #7 | `525912` | MATCH all langs |
| `gemm_matrix.c` | #10 | `6422836` | MATCH all langs |
| `monte_carlo_bs.c` | #11 | `10440246` | MATCH all langs |
| `binary_trees.c` | #2 | `407713` | MATCH all langs (heavier; CI OK at `-O3`) |
| `sha256_blocks.c` | #4 | `-4721506799343634759` | MATCH all langs |

Gates: `size_echo_gates::suite12_*_checksum`.

Until a full multi-lang harness lands, competitive **timing** claims stay on **Suite5**.
Do not claim these C ports equal End’s wall-clock matrix.
