# ADR-0011: Refuse suite12 divergent benchmarks without checksum agreement

Date: 2026-08-23  
Status: **Accepted**

## Context

End’s suite12 publishes multi-language timings for benches `1..12`. Stored End
results (`suite12_results.json`) show **checksum divergence** across languages
for several ids — so End’s “same algorithm” marketing is not an evidence gate.

Rynix already ships checksum-locked C ports under `benchmarks/suite12/` for ids
where peers agree (MATCH). Remaining divergent ids must not become fake ✅ ports.

## Evidence (End peer `suite12_results.json`, 2026-08-21)

| End id | Theme | Peer checksums | Decision |
|--------|-------|----------------|----------|
| **#1** | 3D SDF Raymarcher | End `14694947` ≠ Zig/Rust/Go `17840960` ≠ C `17840942` | **Skip** |
| **#5** | N-body / orbit | End/C/Zig/Rust disagree on stored checksums | **Skip** |
| **#6** | Ring / buffer | End checksum ≠ other langs in stored results | **Skip** |

MATCH ports (in-tree gates): #2 trees, #3 HFT, #4 SHA-256, #7 DNA, #8 JSON,
#9 FSM (C/Zig/Rust/Go; End historically diverges), #10 GEMM, #11 MC, #12 ALU.

## Decision

1. Do **not** add Rynix suite12 ports for #1 / #5 / #6 until a **single**
   reference checksum is agreed (C peer as oracle) and every claimed language
   prints that exact `checksum=` line.
2. Do **not** publish cross-repo millisecond tables against End suite12 for
   divergent ids.
3. Document skips in `benchmarks/suite12/README.md` with this ADR link.
4. Competitive timing claims remain on **Suite5** (C↔Rynix CI checksum gate).

## Consequences

- ROADMAP / END_PEER_GAP may mark “suite12 divergent closed by ADR-0011” ✅
  for the *decision*, never for a stub port.
- Revisit only with a new ADR that names the oracle checksum + in-tree gate.
