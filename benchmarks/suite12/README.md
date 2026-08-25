# Suite12 vs Rynix Suite5 — honesty note (SURPASS E2)

End ships `benchmarks/suite12/` (one binary × benches `1..12`, heavy sims:
raymarch, HFT, N-body, …). Stored End results sometimes **diverge in checksum**
from C/Rust/Go/Zig peers — so End’s “same algorithm” marketing is not an
evidence gate.

## What Rynix does instead

| Harness | Role |
|---------|------|
| [`../suite5/`](../suite5/) | **Gated** 12× integer kernels; CI requires C ↔ Rynix checksum match |
| This folder | **Checksum-locked** C ports + **Phase 12 Wave 5 `.ryx` ports** for MATCH ids |

## Policy (do not weaken)

1. Do **not** publish cross-repo millisecond tables against End suite12 unless
   every language prints the **same** `checksum=` for that bench id.
2. Prefer porting a bench into Suite5 (opaque trip counts + Notes) over cloning
   End’s single-binary spectacle.
3. Spirit analogues (e.g. Suite5 `reduce` vs End #12 ALU mix) must be labeled
   as such in [docs/END_PEER_GAP.md](../../docs/END_PEER_GAP.md).
4. Skip ports where End/C/peers diverge (raymarcher #1, n-body #5, ring #6).
   Closed by [ADR-0011](../../docs/adr/0011-suite12-divergent-benches.md) —
   no stub ports without a shared oracle checksum.
5. **Not an End ms table:** `.ryx` ports below prove checksum parity with the C
   oracle only. They are **not** a wall-clock leaderboard vs End suite12.

## Locked ports (CI)

| File | End id | Locked `checksum=` | Notes |
|------|--------|--------------------|-------|
| `alu_reduction.c` / `alu_reduction.ryx` | #12 | `3370198876750320971` | MATCH; ryx gate `suite12_alu_ryx_checksum` |
| `hft_engine.c` | #3 | `552829538` | MATCH all langs |
| `json_serializer.c` / `json_serializer.ryx` | #8 | `5588438541400559045` | MATCH; ryx gate `suite12_json_ryx_checksum` |
| `fsm_lexer.c` | #9 | `-103069600432064540` | MATCH C/Zig/Rust/Go; End historically diverges |
| `dna_levenshtein.c` | #7 | `525912` | MATCH all langs |
| `gemm_matrix.c` | #10 | `6422836` | MATCH all langs |
| `monte_carlo_bs.c` | #11 | `10440246` | MATCH all langs |
| `binary_trees.c` | #2 | `407713` | MATCH all langs (heavier; CI OK at `-O3`) |
| `sha256_blocks.c` / `sha256_blocks.ryx` | #4 | `-4721506799343634759` | MATCH; ryx gate `suite12_sha256_ryx_checksum` |

Gates: `size_echo_gates::suite12_*_checksum` (C) and `suite12_*_ryx_checksum` (#12/#4/#8).

Until a full multi-lang harness lands, competitive **timing** claims stay on **Suite5**.
Do not claim these C / `.ryx` ports equal End’s wall-clock matrix.
