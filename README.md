# Rynix

**AI-native systems language** — Zero-GC memory, colorless concurrency, canonical
syntax, machine-consumable diagnostics, LLVM backends aiming at sub-1MB binaries.

| Layer | What you get |
|-------|----------------|
| Front-end | Zero-alloc lexer → arena AST → sema → RIR (SSA) |
| Memory | Escape analysis → stack / region / heap + injected `free` |
| Runtime | Fibers, portable TCP, Linux io_uring path, region Vec/Map |
| Tooling | `fmt`, `mcp-serve`, JSON diagnostics, CI + ASan gates |

Docs: [ROADMAP](docs/ROADMAP.md) · [SPEC](docs/SPEC.md) · [ABI](docs/abi.md) · [Diagnostics](docs/diagnostics.md)

---

## Quick start

```sh
cargo build
cargo test --workspace

# Front-end
cargo run -p rynixc -- check testdata/lexer/hello.ryx --explain-alloc
cargo run -p rynixc -- dump-rir testdata/lexer/hello.ryx --opt
cargo run -p rynixc -- fmt testdata/lexer/hello.ryx

# Native binary (needs clang on PATH; MinGW clang preferred on Windows)
cargo run -p rynixc -- build testdata/lexer/hello.ryx -o target/hello --runtime=portable
cargo run -p rynixc -- run testdata/lexer/hello.ryx

# MCP (JSON-RPC 2.0 / Content-Length on stdio)
cargo run -p rynixc -- mcp-serve
```

Windows: `x86_64-pc-windows-gnu` toolchain. Linux: add `--runtime=uring` for the
syscall io_uring path (falls back to portable I/O if ring setup fails).

---

## Repository map

```
crates/
  rynix-span      spans, mmap SourceMap, interner
  rynix-diag      RYX#### + rynix.diag.v1 JSON
  rynix-lexer     zero-alloc total lexer
  rynix-ast       arena AST + canonical formatter
  rynix-parser    recursive descent + Pratt
  rynix-sema      names, types, soft std / smart primitives
  rynix-rir       SSA IR, passes, BCE, escape, interpreter
  rynix-codegen   textual LLVM IR + reachability DCE
  rynixc          CLI driver
rt/               rynix_rt_* (fibers, regions, TCP, uring, Vec/Map)
std/              documented soft prelude surface
testdata/         corpora + directive tests
.github/workflows CI (cargo test + ASan runtime smokes)
docs/             roadmap, spec, ABI, ADRs, schemas
```

---

## Language snapshot (v0.1)

```ryx
## Canonical: def/end, Newline statements, one way to write anything.
def main() -> i64
  let xs = [1, 2, 3]
  let mut sum = 0
  for x in xs
    sum += x
    if sum == 6
      break
    end
  end
  print("ok")
  return sum
end
```

Soft builtins (no import yet): `print`, `sleep_ms`, `yield`, `now_ms`,
`fiber_run`, `vec_new` / `vec_push` / `vec_get` / `vec_len`,
`map_new` / `map_insert` / `map_get` / `map_len`, `tcp_listen` / `tcp_accept` /
`tcp_close`, `tensor(len, […])`, `signal`, `agent`.

---

## Acceptance gates (honest)

| Gate | How we prove it |
|------|-----------------|
| Hello `< 300KiB` | `rynixc` test `size_echo_gates` |
| Fiber scheduler | `rt/tests/fiber_smoke.c` |
| Pipe echo | `rt/tests/echo_smoke.c` |
| TCP echo + RPS floor | `rt/tests/tcp_echo_rps.c` (non-blocking + fibers) |
| Region Vec/Map | collections smoke in `size_echo_gates` |
| Index + BCE | `rynix-rir` `bounds_index` |
| Free inject | `rynix-rir` `free_at` |
| Differential oracle | `rynix-rir` `differential` (interp) |
| Sanitizer | GitHub Actions ASan job on Ubuntu |

**Not claimed:** full liburing SQE park under load vs Go/Tokio; language-level
generics (Vec/Map are i64-monomorphized runtime); inkwell migration (ADR-0005
step 2); lexer 1 GB/s stretch.

---

## CLI surface

`lex` · `parse` · `check` · `dump-rir` · `emit-ll` · `build` · `run` · `test` ·
`fmt` · `mcp-serve`

MCP tools: `diagnostics` / `rynix_check`, `rynix_format`, `rynix_explain_alloc`,
`compile`, `ast_query`, `apply_fix`.

Manifest sketch: [`rynix.toml`](rynix.toml).
