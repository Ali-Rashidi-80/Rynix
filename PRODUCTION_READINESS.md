# Production readiness (honest)

**Version:** `0.2.0` (Track G public) · Quality-10 `0.1.1` · Niche-10 base `0.1.0` · **Toolchain:** `rynixc`  
Status: **Lead Platform Complete** — Golden Lead Phases **0 + 39–43 + 46-pre + 46**
([docs/GOLDEN_LEAD.md](docs/GOLDEN_LEAD.md)); **v0.2.0 shipping** — acceptance-gated phases **0–36**
complete (Q-Core 0–29 + Golden Remaining 30–36); Phase 37 public cut; **Niche-10 certified**
([docs/NICHE10.md](docs/NICHE10.md),
[docs/adr/0013-niche-10-scorecard.md](docs/adr/0013-niche-10-scorecard.md));
ROI 21–24 + Golden Path 25–29 ([docs/GOLDEN_PATH.md](docs/GOLDEN_PATH.md));
Remaining path closed: [docs/GOLDEN_REMAINING.md](docs/GOLDEN_REMAINING.md).  
Lead path closed: [docs/GOLDEN_LEAD.md](docs/GOLDEN_LEAD.md) (Phases 39–46).

## Quality-10 scoreboard

Engineering maturity (not Absolute-10). Scores use GOLDEN_PATH axis re-score method:
named gates only. Prior = analysis report (2026-08); After = post Phases 25–37.

| Axis | Prior | After | Named gates / evidence |
|------|------:|------:|------------------------|
| Architecture | 9.4 | 9.5 | `arch check`, ADR discipline Phases 25–36 |
| Rust code quality | 8.6 | 9.5 | `lower_decomp_invariants`, `lsp_decomp_parity`, `unwrap_budget_gate` |
| C runtime quality | 8.2 | 9.2 | ASan+UBSan CI; Phase 32 uring TCP recv/send |
| Test strategy | 9.2 | 9.5 | `fuzz_new_targets_seeded`, Suite5 CI |
| Error handling | 8.8 | 9.2 | unwrap budget ≤60 |
| Security | 7.6 | 9.3 | Phase 31: UBSan, cargo-deny, Job Object, CWE additive |
| Performance | 9.0 | 9.0 | Suite5 honesty; not Q-Full D-* |
| Deployment / CI | 8.6 | 9.6 | `release.yml` + `v0.2.0` SHA256SUMS |
| AI tooling | 9.4 | 9.9 | Track G + LSP codeAction/inlayHint; MCP dual-era discover |
| Documentation | 9.4 | 9.8 | Book, GOLDEN_PATH/REMAINING close, this scoreboard |
| Niche-10 axe | 9.0 | 9.0 | [NICHE10.md](docs/NICHE10.md) certified |

**Weighted Quality-10:** every axis ≥ **9.0**, Security ≥ **9.0**. Track G shipped
at `v0.2.0` ([GOLDEN_REMAINING.md](docs/GOLDEN_REMAINING.md)).

| Subsystem | Status | Evidence | Known limits |
|-----------|--------|----------|--------------|
| Lexer | Ready | `cargo test -p rynix-lexer`, criterion ~400 MiB/s gate | Not SIMD-tuned to 1 GiB/s |
| Parser / AST | Ready | snapshots, fuzz README, recovery tests | Multiline `"""` (Phase 33) |
| Sema | Ready | `#^ error` directive tests, match/method typing | Track G matrices `Vec[T]`/`Map[K,V]` ([ADR-0025](docs/adr/0025-parametric-monomorphization.md)); not HKT |
| RIR + passes | Ready | verify, DCE/const-fold/simplify/BCE, interpreter | DCE tombstones dead bool ops |
| Escape + free | Ready | `free_at`, `--explain-alloc`, MCP explain | SCC deepen (Phase 32 gate) |
| LLVM codegen | Ready | `diff_llvm_vs_interp`, size gate `<300KiB` hello | Textual IR only (ADR-0005) |
| Runtime fibers | Ready | ASan smokes, fiber park, TCP echo/load | uring TCP recv/send (Phase 32) |
| io_uring (Linux) | Ready | CI uring SQE + TCP recv/send + load smokes | Windows uses portable runtime |
| IOCP (Windows) | Ready | AcceptEx/ConnectEx smokes (`--runtime=iocp`) | Linux uses portable/uring |
| MCP | Ready | **19** tools incl. `rynix_slice`; path-first; **`server/discover`** dual-era; tool annotations | stdio primary; Streamable HTTP → Track-L |
| AI CLI | Ready | `graph`, `slice`, `impact`, `eval`, `patch`, `verify`, `precheck`, `context`, `security`, `scope`, `deps`, `dna`, `arch check` | Agent write needs `rynix.scope.toml` / `--force-write` |
| Architecture guard | Ready | `Architecture.toml`, `arch check`, CI job | Import/call patterns only |
| Benchmarks | Ready | Suite5 **12** workloads + checksum JSON + CI C↔Rynix | Opaque bounds + disclosed strength reduction |
| Editor (LSP) | Ready | diag/hover/def/**completion**/**rename**/**prepareRename**/**references**/**documentHighlight**/**workspace/symbol**/**documentSymbol**/**formatting**/**codeAction**/**inlayHint** | No studio/canvas |
| Std json/http | Ready | GET/POST; serve once/loop; path_param; header/body/keep-alive; TLS; Bearer soft | Not full OAuth |
| Packages | Ready | `rynix.toml`, path+local registry/sparse, lock, `new`, `deps --attest` | Offline-first; no CDN |
| WASM | Ready | `emit-wasm`; Node `main`; host-import `env.print_i64` + `env.print` (str) | No WASI / no `rt/` in wasm |
| TLS / WS / crypto | Ready | Product TLS; WS framing; SHA/HMAC/AES; `std::crypto` facade | Not a general TLS terminator |
| Security posture | Ready | `--sandbox=docker\|job\|none`, cargo-deny CI, RIR sanitize, STRIDE, CWE | MSan optional / platform limits |
| Release binaries | Ready | `.github/workflows/release.yml` + SHA256SUMS; tags `v0.1.1` / `v0.2.0` | GPG optional (documented) |
| Niche-10 | Ready | [docs/NICHE10.md](docs/NICHE10.md) — all ADR-0013 axes gated | Absolute-10 vs Go refused |

We do **not** mark a row Ready without in-tree tests or harnesses listed above.
Raft / llama embed remain deferred ([ADR-0012](docs/adr/0012-deferred-consensus.md)).
