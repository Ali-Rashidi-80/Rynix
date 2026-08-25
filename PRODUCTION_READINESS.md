# Production readiness (honest)

**Version:** `0.1.0` · **Toolchain:** `rynixc`  
Status: **v0.1 shipping core** — acceptance-gated phases 0–20 complete; **Niche-10
certified** ([docs/NICHE10.md](docs/NICHE10.md),
[docs/adr/0013-niche-10-scorecard.md](docs/adr/0013-niche-10-scorecard.md)).

| Subsystem | Status | Evidence | Known limits |
|-----------|--------|----------|--------------|
| Lexer | Ready | `cargo test -p rynix-lexer`, criterion ~400 MiB/s gate | Not SIMD-tuned to 1 GiB/s |
| Parser / AST | Ready | snapshots, fuzz README, recovery tests | No multiline strings |
| Sema | Ready | `#^ error` directive tests, match/method typing | Mono `Vec[i64]`/`Map[i64,i64]` only ([ADR-0006](docs/adr/0006-monomorphized-collections.md), [ADR-0014](docs/adr/0014-mono-collections-niche10.md)) |
| RIR + passes | Ready | verify, DCE/const-fold/simplify/BCE, interpreter | DCE tombstones dead bool ops |
| Escape + free | Ready | `free_at`, `--explain-alloc`, MCP explain | Intraprocedural-first; conservative FFI |
| LLVM codegen | Ready | `diff_llvm_vs_interp`, size gate `<300KiB` hello | Textual IR only (ADR-0005) |
| Runtime fibers | Ready | ASan smokes, fiber park, TCP echo/load | recv/send poll path on uring builds |
| io_uring (Linux) | Ready | CI uring SQE + TCP/load smokes | Windows uses portable runtime |
| IOCP (Windows) | Ready | AcceptEx/ConnectEx smokes (`--runtime=iocp`) | Linux uses portable/uring |
| MCP | Ready | **18** tools; path-first `rynix_graph` / `rynix_impact` / `rynix_precheck` (fail-closed) | stdio JSON-RPC only |
| AI CLI | Ready | `graph`, `slice`, `impact`, `eval`, `patch`, `verify`, `precheck`, `context`, `security`, `scope`, `deps`, `dna`, `arch check` | Agent write needs `rynix.scope.toml` / `--force-write` |
| Architecture guard | Ready | `Architecture.toml`, `arch check`, CI job | Import/call patterns only |
| Benchmarks | Ready | Suite5 **12** workloads + checksum JSON + CI C↔Rynix | Opaque bounds + disclosed strength reduction; not End suite12 sims |
| Editor (LSP) | Ready | `lsp-serve` diag/hover/def/**completion**/**rename**; VS Code ext + **CodeLens** (check/alloc/impact) | No studio/canvas ([ADR-0007](docs/adr/0007-deferred-ui-frameworks.md)) |
| Std json/http | Ready | GET/POST; serve once/loop 1–3 paths; path_param; header/body/keep-alive; TLS product path | Not a full framework / nginx RPS |
| Packages | Ready | `rynix.toml`, path+local registry/sparse, lock, `new`, `deps --attest` → `rynix.attest.v1` **local digest** (not Sigstore/Rekor) | Offline-first; no CDN registry |
| WASM | Ready | `emit-ll --target=wasm32-unknown-unknown`; `emit-wasm` → `\0asm`; Node runs `main`; host-import `env.print_i64` | No WASI / no `rt/` in wasm |
| TLS / WS / crypto | Ready | Product TLS serve/client smoke; WS framing; SHA/HMAC/AES KAT | Not a general TLS terminator |
| Release binaries | Ready | `.github/workflows/release.yml` + SHA256SUMS | GPG optional (documented) |
| Niche-10 | Ready | [docs/NICHE10.md](docs/NICHE10.md) — all ADR-0013 axes gated | Absolute-10 vs Go refused |

We do **not** mark a row Ready without in-tree tests or harnesses listed above.
Raft / llama embed remain deferred ([ADR-0012](docs/adr/0012-deferred-consensus.md)).
