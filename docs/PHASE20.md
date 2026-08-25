# Phase 20 — WASM host-import + package/INSTALL polish + Niche-10 certify

**Status:** **Phase 20 complete** (2026-08-25)  
**Certification:** [NICHE10.md](NICHE10.md) · [ADR-0013](adr/0013-niche-10-scorecard.md)

## Gates

| Wave | Gate | Theme |
|------|------|--------|
| A | `emit_wasm_host_print_i64` | freestanding wasm + `env.print_i64` host import (Node) |
| B | `package_ux_new_deps_attest` | `new` + `deps --attest` UX |
| C | `install_one_path_clang_win_linux` | INSTALL.md one-path clang |
| D | `niche10_scorecard_links_gates` | Niche-10 scorecard evidence links |

## Refuse

Full WASI, Absolute-10 vs Go, Raft/llama theater ([ADR-0012](adr/0012-deferred-consensus.md)).
