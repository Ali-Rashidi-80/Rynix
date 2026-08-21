# Rynix

Rynix is an experimental AI-native systems programming language: a Zero-GC,
colorless-concurrency backend language with a canonical (one-way) syntax,
structured JSON diagnostics, and an LLVM backend targeting sub-1MB binaries.

The compiler front-end is written in Rust with a zero-allocation philosophy
(memory-mapped sources, span-based tokens, arena-allocated ASTs).

## Status

Early development. See [docs/ROADMAP.md](docs/ROADMAP.md) for the full
phase-by-phase plan and [docs/SPEC.md](docs/SPEC.md) for the language
specification (v0.1 draft).

## Layout

- `crates/rynix-span` — spans, source map (mmap), string interner
- `crates/rynix-diag` — structured diagnostics (`RYX####` codes, fixes with confidence, JSON)
- `crates/rynix-lexer` — zero-allocation total lexer
- `crates/rynix-ast` — arena-allocated AST (Phase 2)
- `crates/rynix-parser` — recursive-descent + Pratt parser
- `crates/rynix-sema` — name resolution and type checking
- `crates/rynix-rir` — canonical SSA IR (block args, passes, interpreter)
- `crates/rynixc` — compiler driver CLI
- `docs/` — roadmap, spec, diagnostics registry, ADRs
- `testdata/` — `.ryx` corpora for snapshot tests and benchmarks
- `fuzz/` — cargo-fuzz targets (run on Linux/WSL2)

## Building

```sh
cargo build
cargo test
cargo bench -p rynix-lexer   # lexer throughput
```

On Windows the repository uses the `x86_64-pc-windows-gnu` toolchain (no
Visual Studio required). Everything through code generation is
platform-portable; the fiber/io_uring runtime (Phase 8) targets Linux.

## Trying the front-end

```sh
cargo run -p rynixc -- lex testdata/lexer/hello.ryx --dump-tokens
cargo run -p rynixc -- parse testdata/lexer/functions.ryx --dump-ast
cargo run -p rynixc -- check testdata/lexer/errors.ryx --error-format=json
cargo run -p rynixc -- check testdata/lexer/hello.ryx --explain-alloc
cargo run -p rynixc -- dump-rir testdata/lexer/hello.ryx
cargo run -p rynixc -- dump-rir testdata/lexer/functions.ryx --opt --escape
```

Machine-readable diagnostics follow [`docs/schemas/rynix.diag.v1.json`](docs/schemas/rynix.diag.v1.json).
