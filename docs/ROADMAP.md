# Rynix Development Roadmap

Rynix is a systems/backend programming language designed to be AI-native
first: canonical syntax (one way to do anything), machine-consumable
diagnostics, deterministic Zero-GC memory management, colorless concurrency
on io_uring, and an LLVM backend producing sub-1MB binaries.

This roadmap is atomic and phase-gated: no phase closes without tests and an
explicit acceptance criterion. Irreversible decisions are recorded as ADRs in
[docs/adr/](adr/).

## Naming and conventions

- Compiler binary: `rynixc`. Source extension: `.ryx`. Textual IR: `.rir`.
- Diagnostic codes: `RYX####` (see [diagnostics.md](diagnostics.md)).
- One atomic step = one commit. English docs; canonical formatting only.
- Development platform: Windows (gnu toolchain) is fully supported through
  codegen; the runtime phase (io_uring) develops/tests on WSL2 or Docker and
  CI runs on Linux.

## Pipeline overview

```
.ryx (mmap) -> Lexer (zero-alloc tokens) -> Parser (arena AST)
    -> Sema (names + types) -> RIR (canonical SSA)
    -> Escape Analysis + Region Inference + injected free
    -> LLVM IR + ThinLTO + whole-program DCE -> binary (<1MB) + rynix-rt
Every stage emits structured diagnostics (JSON / MCP).
```

## Phase 0 — Workspace scaffold (done first)

- Cargo workspace, stable toolchain pin, pedantic lints,
  `unsafe_op_in_unsafe_fn = deny`, release profile `lto=fat`,
  `codegen-units=1`, `panic=abort`.
- Crates: `rynix-span`, `rynix-diag`, `rynix-lexer`, `rynix-ast`,
  `rynix-parser`, `rynixc`. Dependency policy: minimal, ADR-gated
  (`memmap2`, `memchr`, `rustc-hash`, `bumpalo`, `serde{,_json}` in diag only).
- Acceptance: green `cargo build`/`cargo test`, first commit, docs in place.

## Phase 1 — Zero-allocation lexer

Data structures:

- `Span { lo: u32, hi: u32 }` — 8-byte Copy, global offset space (ADR-0003).
- `Token { kind: TokenKind, span: Span }` — 12-byte Copy.
- `TokenKind` — `#[repr(u8)]`, 70 variants (3 literals, `Ident`, 26 keywords,
  4 reserved keywords, 30 punctuation, 6 structural).
- `Cursor<'src>` — lazy, total, allocation-free lexer over `&[u8]` from a
  memory-mapped `SourceFile`.

Algorithm: static 256-entry first-byte dispatch table; `memchr`-accelerated
string/comment scanning; keyword recognition via length+bytes match;
ASCII-only identifiers (ADR-0002); `Newline` is a real token (statement
terminator), the parser ignores it inside brackets.

Errors are structured from day one: `RYX0001..RYX0006` with confidence-scored
fixes (e.g. unterminated string suggests inserting `"` at end-of-line).

Testing (six layers): unit tests per token kind and boundary; insta snapshot
corpus (`testdata/lexer/*.ryx`); proptest invariants (perfect tiling: token
spans partition the input byte-exactly; totality; non-empty non-EOF tokens);
a zero-allocation counter test (custom `GlobalAlloc` proves 0 heap
allocations on a clean corpus); cargo-fuzz target (Linux/CI); criterion
throughput benches with committed baseline.

Acceptance: `rynixc lex file.ryx --dump-tokens --error-format=json` works
from an mmap'd file; all tests green; throughput >= 400 MB/s single-core
initially (stretch: 1 GB/s).

## Phase 2 — Parser and arena AST

- `AstArena` (bumpalo newtype): all nodes `&'arena`, lists `&'arena [T]`,
  no `Box`/`Rc`/`Drop`/`String` in nodes; identifiers are interned `Symbol`s;
  `NodeId(u32)` for SoA side tables in sema.
- v0.1 nodes: `Module, FnDef, StructDef, EnumDef, TypeAlias, Import`;
  `Let, ExprStmt, Return, Break, Continue`; expressions (literals, path,
  unary, binary, call, method call, index, field, if/elif/else, loop, for,
  block); types (path, ref, slice, fn).
- Hand-written recursive descent + Pratt (precedence:
  `or < and < not < comparison < range < additive < multiplicative < unary <
  as < postfix`). Comparisons are non-associative (canonical).
- Total parser: `Error` nodes + panic-mode sync at `{Newline, end, def, struct}`.
- Tests: s-expression AST snapshots, error-recovery snapshots, pretty-print
  round-trip property (printer later becomes `rynix fmt`), fuzz.

## Phase 3 — JSON diagnostics and MCP schema

- Dual renderers: human (annotated snippets) and NDJSON `rynix.diag.v1`
  (code, spans with line/col, fixes with confidence, compiler stage) plus a
  JSON Schema document and golden validation tests.
- `rynixc check` wires lexer+parser diagnostics. The full JSON-RPC 2.0
  `rynixc mcp-serve` (tools: compile, diagnostics, ast_query, apply_fix)
  lands in Phase 9; the schema is frozen here.

## Phase 4 — Semantic analysis: names and types

- Scope tree (`IndexVec<ScopeId, Scope>`), two-pass resolution (items, then
  bodies), `DefId(u32)`.
- `TypeCtx` with hash-consing (`TypeId(u32)`): ints/floats, bool, str, unit,
  never, nominal struct/enum, ref, slice, fn. Literal defaults: `i64`/`f64`.
- Function-local inference only (unification); signatures always explicit —
  required for interprocedural analysis and LLM predictability.
- Tests: type dumps; comment-directive tests (`#^ error RYX2xxx`).

## Phase 5 — RIR: canonical SSA IR ✅

- SoA `Function { blocks, insts }` with block arguments (Cranelift-style)
  instead of phi nodes; ~25 instructions including `alloc{site_id}` (the unit
  of escape reasoning), calls, branches. Locals lower via alloc/load/store
  (Braun-ready sealed blocks; full on-the-fly SSA values deferred where
  mutable slots dominate).
- Structural verifier between passes; textual `.rir` printer + subset parser;
  baseline passes: DCE, const-fold, simplify-cfg. Interval range analysis for
  bounds-check elimination deferred to when indexing lands in lowering.
- Small RIR interpreter as a differential-testing oracle for codegen.
- CLI: `rynixc dump-rir [--opt]`.

## Phase 6 — Escape analysis and region inference (the Zero-GC core) ✅

- Per-allocation-site lattice `NoEscape < ArgEscape < RegionEscape <
  GlobalEscape`; mapping: NoEscape -> stack; Arg/RegionEscape -> implicit
  bump arenas (no `region` keyword); GlobalEscape -> heap with
  compiler-injected `free` via last-use (`Inst::Free`).
- Intraprocedural points-to on SSA; bottom-up interprocedural summaries over
  the call graph with SCC fixpoints; `call_ext` conservative except benign
  builtins (`print`/`println`/`assert`).
- `region_create` / `region_reset` injected at function entry and loop
  headers when any site is region-placed.
- Transparency: `rynixc check --explain-alloc` (human + JSON
  `rynix.alloc.v1`); `rynixc dump-rir --escape`.
- Tests: `#^ alloc: stack|region|heap` directives; unit tests for heap/region.

## Phase 7 — LLVM backend and sub-1MB binaries ✅

- Step 1: emit textual LLVM IR (no LLVM linkage — Windows-friendly), link via
  `clang -O3 -flto=thin -ffunction-sections -Wl,--gc-sections` plus
  [`rt/portable.c`](../rt/portable.c). Step 2 (later): inkwell.
- Whole-program reachability DCE at the RIR level — only functions reachable
  from `main` are emitted (`rynix-codegen::prune_unreachable`).
- [`docs/abi.md`](abi.md) documents the `rynix_rt_*` symbol set.
- CLI: `rynixc emit-ll`, `rynixc build` (requires `clang` on PATH).
- Tests: `.ll` pattern tests (print, no heap for stack locals, dead-fn DCE).

## Phase 8 — Runtime: fibers + io_uring (colorless concurrency)

- `rynix-rt` staticlib (C ABI): x86_64 SysV context switch in inline asm
  (callee-saved + rsp; target < 30ns); fiber stacks mmap'd with guard page,
  fixed 256KB in v0.
- Thread-per-core scheduler: one event loop + private io_uring per core
  (`io-uring` crate), local run queue, no work stealing in v0, MPSC injector
  for cross-core spawn, park via `io_uring_enter(min_complete=1)`.
- Colorless: blocking-looking stdlib calls (read/accept/sleep) lower to
  SQE submit + fiber yield; no async/await anywhere in the language.
- Environments: WSL2 Ubuntu or Docker for dev/test; Linux CI. A portable
  blocking backend (`--runtime=portable`) keeps the Windows dev loop alive.
- Tests: context-switch microbench; echo-server load test (rewrk) vs
  Go/Tokio baselines; ASan/TSan runs; fiber-leak assertion at exit.

## Phase 9 — Stdlib, tooling, AI features

- Minimal std on region allocators: core (Vec/Map/str), io, fs, net, time,
  json.
- Full CLI: `rynix build/run/test/fmt` + `rynix.toml`; canonical formatter
  with zero configuration.
- Full `rynixc mcp-serve` (JSON-RPC 2.0); Presburger bounds-check
  elimination; smart-primitive experiments (`tensor` with compile-time shape
  checking, `signal`, `agent`).

## Milestones

- M0: workspace + docs committed.
- M1: `rynixc lex` at target throughput with zero allocations proven.
- M2: parse + AST dumps with error recovery.
- M3: JSON diagnostics frozen (`rynix.diag.v1`).
- M4: type checking.
- M5: RIR + interpreter oracle.
- M6: escape/region analysis with `--explain-alloc`.
- M7: native binaries via LLVM within size gates.
- M8: fiber/io_uring echo server at target RPS.
- M9: std + MCP server + fmt.
