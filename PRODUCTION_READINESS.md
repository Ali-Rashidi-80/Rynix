# Production readiness (honest)

**Version:** `0.1.0` · **Toolchain:** `rynixc`  
Status: **v0.1 shipping core** — acceptance-gated phases 0–10 complete.

| Subsystem | Status | Evidence | Known limits |
|-----------|--------|----------|--------------|
| Lexer | Ready | `cargo test -p rynix-lexer`, criterion ~400 MiB/s gate | Not SIMD-tuned to 1 GiB/s |
| Parser / AST | Ready | snapshots, fuzz README, recovery tests | No multiline strings |
| Sema | Ready | `#^ error` directive tests, match/method typing | Mono `Vec[i64]`/`Map[i64,i64]` only |
| RIR + passes | Ready | verify, DCE/const-fold/simplify/BCE, interpreter | DCE tombstones dead bool ops |
| Escape + free | Ready | `free_at`, `--explain-alloc`, MCP explain | Intraprocedural-first; conservative FFI |
| LLVM codegen | Ready | `diff_llvm_vs_interp`, size gate `<300KiB` hello | Textual IR only (ADR-0005) |
| Runtime fibers | Ready | ASan smokes, fiber park, TCP echo/load | recv/send poll path on uring builds |
| io_uring (Linux) | Ready | CI uring SQE + TCP/load smokes | Windows uses portable runtime |
| MCP | Ready | 11 tools incl. graph/impact/eval/arch | stdio JSON-RPC only |
| AI CLI | Ready | `graph`, `slice`, `impact`, `eval`, `patch`, `arch check` | — |
| Architecture guard | Ready | `Architecture.toml`, `arch check`, CI job | Import/call patterns only |
| Benchmarks | Ready | Suite5 **12** workloads + checksum JSON + CI | Not End-style heavy sims (see benchmarks/README) |
| Editor (LSP) | Ready | `lsp-serve` (diag/hover/def), VS Code ext | No CodeLens/studio |
| Std json/http | Ready | `json_get_i64` (unit + example e2e), `http_get_json_i64` (sema/LLVM + connect-fail smoke) | Minimal JSON (int fields); no live HTTP server in CI |
| Release binaries | Ready | `.github/workflows/release.yml` + SHA256SUMS | GPG optional (documented) |

We do **not** mark a row Ready without in-tree tests or harnesses listed above.
