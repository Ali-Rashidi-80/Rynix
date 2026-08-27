# Production readiness (honest)

**Version:** `0.1.1` (Quality-10 public) · Niche-10 base `0.1.0` · **Toolchain:** `rynixc`  
Status: **v0.1.1 shipping** — acceptance-gated phases **0–29** (Q-Core; includes **0–24**) complete;
Phase 30 public cut; **Niche-10 certified** ([docs/NICHE10.md](docs/NICHE10.md),
[docs/adr/0013-niche-10-scorecard.md](docs/adr/0013-niche-10-scorecard.md));
ROI 21–24 + Golden Path 25–29 ([docs/GOLDEN_PATH.md](docs/GOLDEN_PATH.md));
remaining: [docs/GOLDEN_REMAINING.md](docs/GOLDEN_REMAINING.md).

## Quality-10 scoreboard

Engineering maturity (not Absolute-10). Scores use GOLDEN_PATH axis re-score method:
named gates only. Prior = analysis report (2026-08); After = post Phases 25–30.

| Axis | Prior | After | Named gates / evidence |
|------|------:|------:|------------------------|
| Architecture | 9.4 | 9.5 | `arch check`, ADR discipline Phases 25–29 |
| Rust code quality | 8.6 | 9.5 | `lower_decomp_invariants`, `lsp_decomp_parity`, `unwrap_budget_gate` |
| C runtime quality | 8.2 | 9.0 | ASan `sanitizer-rt` CI; MSan/UBSan → Phase 31 behavioral |
| Test strategy | 9.2 | 9.5 | `fuzz_new_targets_seeded`, Suite5 CI |
| Error handling | 8.8 | 9.2 | unwrap budget ≤60 |
| Security | 7.6 | 9.0 | `sandbox_docker_smoke`, `sanitize_rejects_exec`, threat model; harden → Phase 31 |
| Performance | 9.0 | 9.0 | Suite5 honesty; not Q-Full D-* |
| Deployment / CI | 8.6 | 9.5 | `release.yml` + `v0.1.1` SHA256SUMS |
| AI tooling | 9.4 | 9.6 | `document_symbol_lists_fn`, LSP formatting, MCP `rynix_slice` |
| Documentation | 9.4 | 9.7 | Book skeleton, GOLDEN_PATH/REMAINING, this scoreboard |
| Niche-10 axe | 9.0 | 9.0 | [NICHE10.md](docs/NICHE10.md) certified |

**Weighted Quality-10:** every axis ≥ **9.0**, Security ≥ **9.0**. Further Security
behavioral harden is Phase 31 ([GOLDEN_REMAINING.md](docs/GOLDEN_REMAINING.md)).

| Subsystem | Status | Evidence | Known limits |
|-----------|--------|----------|--------------|
| Lexer | Ready | `cargo test -p rynix-lexer`, criterion ~400 MiB/s gate | Not SIMD-tuned to 1 GiB/s |
| Parser / AST | Ready | snapshots, fuzz README, recovery tests | Multiline → Phase 33 |
| Sema | Ready | `#^ error` directive tests, match/method typing | Mono `Vec[i64]`/`Vec[str]`/`Map[i64,i64]`/`Map[str,i64]`/`Map[str,str]` ([ADR-0018](docs/adr/0018-map-str-str-mono.md)); not parametric |
| RIR + passes | Ready | verify, DCE/const-fold/simplify/BCE, interpreter | DCE tombstones dead bool ops |
| Escape + free | Ready | `free_at`, `--explain-alloc`, MCP explain | Intraprocedural-first; Phase 32 SCC deepen |
| LLVM codegen | Ready | `diff_llvm_vs_interp`, size gate `<300KiB` hello | Textual IR only (ADR-0005) |
| Runtime fibers | Ready | ASan smokes, fiber park, TCP echo/load | recv/send poll path → Phase 32 uring |
| io_uring (Linux) | Ready | CI uring SQE + TCP/load smokes | Windows uses portable runtime |
| IOCP (Windows) | Ready | AcceptEx/ConnectEx smokes (`--runtime=iocp`) | Linux uses portable/uring |
| MCP | Ready | **19** tools incl. `rynix_slice`; path-first fail-closed | stdio JSON-RPC only |
| AI CLI | Ready | `graph`, `slice`, `impact`, `eval`, `patch`, `verify`, `precheck`, `context`, `security`, `scope`, `deps`, `dna`, `arch check` | Agent write needs `rynix.scope.toml` / `--force-write` |
| Architecture guard | Ready | `Architecture.toml`, `arch check`, CI job | Import/call patterns only |
| Benchmarks | Ready | Suite5 **12** workloads + checksum JSON + CI C↔Rynix | Opaque bounds + disclosed strength reduction |
| Editor (LSP) | Ready | diag/hover/def/**completion**/**rename**/**references**/**workspace/symbol**/**documentSymbol**/**formatting** | No studio/canvas |
| Std json/http | Ready | GET/POST; serve once/loop; path_param; header/body/keep-alive; TLS | Bearer soft → Phase 32 |
| Packages | Ready | `rynix.toml`, path+local registry/sparse, lock, `new`, `deps --attest` | Offline-first; no CDN |
| WASM | Ready | `emit-wasm`; Node `main`; host-import `env.print_i64` + `env.print` (str) | No WASI / no `rt/` in wasm |
| TLS / WS / crypto | Ready | Product TLS; WS framing; SHA/HMAC/AES; `std::crypto` facade | Not a general TLS terminator |
| Security posture | Ready | `--sandbox=docker\|none`, RIR sanitize, STRIDE, CWE matrix | Job Object / cargo-deny → Phase 31 |
| Release binaries | Ready | `.github/workflows/release.yml` + SHA256SUMS; tag `v0.1.1` | GPG optional (documented) |
| Niche-10 | Ready | [docs/NICHE10.md](docs/NICHE10.md) — all ADR-0013 axes gated | Absolute-10 vs Go refused |

We do **not** mark a row Ready without in-tree tests or harnesses listed above.
Raft / llama embed remain deferred ([ADR-0012](docs/adr/0012-deferred-consensus.md)).
