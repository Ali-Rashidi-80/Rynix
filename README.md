![Rynix — AI-native systems language](assets/logo.png)

# Rynix

**AI-native systems language** — canonical syntax, Zero-GC escape path, colorless
fibers, machine-readable diagnostics, textual LLVM backend, **honest** benchmarks.

`.ryx` · `rynixc` · phases **0–10** gated · [Roadmap](docs/ROADMAP.md)

![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF.svg)
![Version](https://img.shields.io/badge/version-0.1.0-3fb950.svg)
![Rust](https://img.shields.io/badge/Rust-1.98+-orange.svg)
![Phases](https://img.shields.io/badge/phases-0--10%20gated-147A8A.svg)
![Memory](https://img.shields.io/badge/memory-Zero--GC%20escape-0B3D4A.svg)
![AI](https://img.shields.io/badge/AI-MCP%20+%20LSP%20+%20JSON-3ECFB2.svg)

[What is Rynix?](#what-is-rynix) · [Why Rynix](#why-rynix) · [What Rynix is NOT](#what-rynix-is-not) · [vs End](#vs-end-irMahoend) · [Quick start](#quick-start) · [Pipeline](#compiler-pipeline) · [Runtime](#runtime--fibers) · [Memory](#memory-model) · [Tooling](#tooling-surface) · [Benchmarks](#benchmarks) · [FAQ](#faq-quick-fixes) · [Install](INSTALL.md) · [Compare](docs/COMPARE.md) · [End gap analysis](docs/END_PEER_GAP.md)

---



## What is Rynix?

Rynix is a systems language built for **humans and agents**: one canonical spelling
per construct (`def`/`end`, newline statements), structured `rynix.diag.v1` JSON,
and CLI/MCP/LSP surfaces agents can call without scraping stdout.


| Field        | Detail                                                                                      |
| ------------ | ------------------------------------------------------------------------------------------- |
| **Version**  | `0.1.0` — phases **0–10** acceptance-gated ([ROADMAP](docs/ROADMAP.md))                     |
| **Compiler** | Rust workspace (`crates/`) — MSRV **1.98** ([`Cargo.toml`](Cargo.toml))                     |
| **Runtime**  | C (`rt/`) — fibers, TCP, json/http, io_uring (Linux)                                        |
| **Backend**  | Textual LLVM IR → clang ThinLTO ([ADR-0005](docs/adr/0005-textual-llvm-ir-first.md))         |
| **Proof**    | In-tree tests + CI — [PRODUCTION_READINESS](PRODUCTION_READINESS.md)                        |


Zig/Go/C/Rust in [`benchmarks/suite5/`](benchmarks/suite5/) are **peer workload implementations**
(the same 12 algorithms compiled with each toolchain). The **compiler** is Rust, like
[End](https://github.com/IrMaho/End)’s `endc`. Optional 6th peer: **End** when `endc` and
`.end` ports exist — see [`END_INTEGRATION.md`](benchmarks/suite5/END_INTEGRATION.md).

---

## What Rynix is NOT

| Misread | Reality |
|--------|---------|
| ❌ Only a benchmark stunt | ✅ Full compiler pipeline + runtime + LSP/MCP (phases 0–10 gated) |
| ❌ A Rust or Zig clone | ✅ Own syntax (`.ryx`), RIR, escape model, fiber runtime |
| ❌ Beating [End](https://github.com/IrMaho/End) on every axis today | ✅ Stronger **checksum/diff gates**; End leads **frameworks, editor UX, suite12 spectacle** — [`docs/END_PEER_GAP.md`](docs/END_PEER_GAP.md) |
| ❌ Same programs as End suite12 | ✅ Suite5 uses **different**, lighter integer kernels; End #12 ≠ our `reduce.ryx` |
| ❌ “Best repo in history” | ✅ **Auditable** claims: test or CI before ✅ |

---

## vs End ([IrMaho/End](https://github.com/IrMaho/End))

Same thesis (AI-native, Zero-GC, systems backend). Different maturity shape:

| | End | Rynix |
|--|-----|-------|
| Benchmarks | suite12: SDF, HFT, SHA, N-body, … | Suite5: 12 integer microkernels + **CI checksum gate** |
| Languages in matrix | End, C, Rust, Go, Zig | Rynix, C, Rust, Go, Zig, **End** (when `endc` present) |
| Agent contracts / UI / EndHyper | Broad (per End docs) | Deferred / narrow std — see gap doc |
| Proof style | Checksum per bench | **C↔Rynix CI** + LLVM↔interp + escape explain |

Full gap backlog (P0–P3): **[docs/END_PEER_GAP.md](docs/END_PEER_GAP.md)**.

---

## Domain maturity (honest)

Where End markets “every domain,” Rynix statuses are **evidence-gated** (test/CI or deferred ADR):

| Domain | Status | Evidence / pointer |
|--------|--------|-------------------|
| Systems / native binaries | 🟢 Shipping | LLVM + ThinLTO, `size_echo_gates` |
| Backend / TCP | 🟢 Shipping | fiber TCP, bakeoff docs, ASan CI |
| AI-native tooling | 🟢 Shipping | MCP 11 tools, graph/impact/eval schemas |
| Editor (VS Code + LSP) | 🟢 Shipping | `editors/vscode/`, `lsp-serve` (no CodeLens yet) |
| Memory / Zero-GC | 🟢 Shipping | escape → stack/region/heap, `--explain-alloc` |
| Microbench matrix | 🟢 Shipping | Suite5 × 5–6 langs, C↔Rynix CI gate |
| HTTP / JSON helpers | 🔵 Narrow | `json_get_i64`, `http_get_json_i64` smoke |
| Web frameworks / UI canvas | ⚪ Deferred | [ADR-0007](docs/adr/0007-deferred-ui-frameworks.md) |
| C11 backend | ⚪ Deferred | [ADR-0008](docs/adr/0008-deferred-c11-backend.md) |
| Agent contract DSL | ⚪ Gap | [END_PEER_GAP.md](docs/END_PEER_GAP.md) P1 |
| WASM / mobile | ⚪ Out of scope v0.1 | ROADMAP |

---

## Why Rynix

Rynix optimizes for **agent-verifiable** delivery: every roadmap ✅ maps to a test or CI job;
benchmarks gate **checksums before milliseconds**; diagnostics and graph/impact/eval export JSON
schemas agents can consume without scraping.

| Pillar                  | Shipping today                                   | Proof                                            |
| ----------------------- | ------------------------------------------------ | ------------------------------------------------ |
| **Canonical syntax**    | `def`/`end`, newline statements                  | `[docs/SPEC.md](docs/SPEC.md)`, parser snapshots |
| **Zero-GC path**        | Escape → stack / region / heap + injected `free` | `--explain-alloc`, MCP `rynix_explain_alloc`     |
| **Colorless I/O**       | Fibers + `PARKED`; io_uring harvest on Linux     | `rt/tests/`, ASan CI                             |
| **AI-native toolchain** | NDJSON diags, MCP (11 tools), graph/impact/eval  | `[docs/schemas/](docs/schemas/)`, `agent_cli`    |
| **Editor + arch guard** | VS Code + LSP; `Architecture.toml`               | `phase10_gates`, `editors/vscode/`               |
| **Small binaries**      | Hello under **300 KiB** (clang gate)             | `size_echo_gates`                                |


> **vs [End](https://github.com/IrMaho/End):** Rynix leads on **test-gated correctness** and
> several Suite5 rows vs C/Rust/Go/Zig; End leads **README/framework/editor breadth** —
> [`docs/END_PEER_GAP.md`](docs/END_PEER_GAP.md) (honest, not a marketing win claim).

---

## Compiler pipeline

### ASCII (renders everywhere)

```text
  .ryx source
       │
       ▼
  ┌─────────┐    ┌──────────────┐    ┌──────┐    ┌─────────┐
  │  Lexer  │───▶│ Parser / AST │───▶│ Sema │───▶│ RIR SSA │
  └─────────┘    └──────────────┘    └──┬───┘    └────┬────┘
       │                 │               │             │
       │                 └───────────────┴─────────────┤
       │                         rynix.diag.v1 ◀───────┤
       │                                             ▼
       │                              ┌──────────────────────────┐
       │                              │ Escape + region + free   │
       │                              └────────────┬─────────────┘
       │                                           ▼
       │                              ┌──────────────────────────┐
       │                              │ LLVM IR (.ll) + ThinLTO    │
       │                              └────────────┬─────────────┘
       │                                           ▼
       └──────────────────────────────▶ binary + rynix_rt (C)
```

### Mermaid (GitHub / compatible viewers)

```mermaid
flowchart TB
  subgraph compile["Compile path"]
    SRC[".ryx source"] --> LEX["Lexer"]
    LEX --> PAR["Parser / AST"]
    PAR --> SEM["Sema"]
    SEM --> RIR["RIR SSA"]
    RIR --> ESC["Escape + free inject"]
    ESC --> LLVM["LLVM IR"]
    LLVM --> BIN["Binary + rynix_rt"]
  end
  subgraph agent["Agent surfaces"]
    PAR -.-> DIAG["rynix.diag.v1"]
    SEM -.-> DIAG
    SEM --> GRAPH["graph / slice / impact"]
    RIR --> DUMP["dump-rir / emit-ll"]
  end
```



### `rynixc` command surface

```text
Core:     lex · parse · check · dump-rir [--opt] · emit-ll · build · run · test · fmt
Agent:    graph · slice · impact · eval · patch · arch check
Servers:  mcp-serve · lsp-serve
Config:   rynix.toml (optional project file)
```

---

## Runtime & fibers

Blocking-looking std calls lower to **fiber yield + PARKED**; Linux builds can
harvest **io_uring** completions inside `rynix_rt_run`.

```text
  main thread                         fiber A              fiber B
      │                                  │                    │
      ├─ rynix_rt_run() ◀── scheduler ────┤                    │
      │       │                          │                    │
      │       ├─ tcp_recv (would block)  │                    │
      │       │      └─ PARKED ─────────▶│                    │
      │       ├─ run ready fiber ────────────────────────────▶│
      │       └─ io_uring CQ harvest (Linux, --runtime=uring) │
      │                                  │                    │
      └─ resume on I/O complete ◀────────┴────────────────────┘
```


| Runtime  | Flag                 | Platform                               |
| -------- | -------------------- | -------------------------------------- |
| Portable | `--runtime=portable` | Windows (default), Linux fallback      |
| io_uring | `--runtime=uring`    | Linux when built with `RYNIX_RT_URING` |


ABI reference: `[docs/abi.md](docs/abi.md)`

---

## Memory model

Escape analysis assigns each allocation site a tier; the compiler injects `free`
where heap escape is proven.

```text
  NoEscape ──────▶ stack slot
  ArgEscape ─────▶ caller region / bump arena
  RegionEscape ──▶ scoped region (loop / handler)
  GlobalEscape ──▶ heap + compiler-injected free
```

```mermaid
flowchart LR
  ALLOC["allocation site"] --> EA["escape analysis"]
  EA --> NE["NoEscape → stack"]
  EA --> AE["ArgEscape → caller region"]
  EA --> RE["RegionEscape → scoped region"]
  EA --> GE["GlobalEscape → heap"]
  GE --> FREE["compiler-injected free"]
```

```sh
rynixc check file.ryx --explain-alloc --error-format=json
```

---

## Quick start

### Install

```sh
# Unix
chmod +x INSTALL.sh && ./INSTALL.sh

# Windows (PowerShell)
.\install.ps1

# Manual
cargo install --path crates/rynixc --force
```

**Prerequisites:** Rust **1.98+** (see [`Cargo.toml`](Cargo.toml) / [`rust-toolchain.toml`](rust-toolchain.toml)), `clang` on `PATH`.
Windows: use `--runtime=portable` and MinGW `x86_64-w64-mingw32-clang` + `x86_64-pc-windows-gnu`.
Linux: optional `--runtime=uring` when built with `RYNIX_RT_URING`. Details: [INSTALL.md](INSTALL.md).

### First run

```sh
cargo test --workspace

rynixc run examples/01_hello.ryx
rynixc check examples/03_vec.ryx --explain-alloc --error-format=json
rynixc graph examples/02_match_loop.ryx
rynixc arch check
rynixc build examples/03_vec.ryx -o target/ex_vec --runtime=portable
rynixc run examples/05_http_json.ryx    # json_get_i64 → stdout 42
```

### Project config (`rynix.toml`)

Optional manifest beside your sources (see repo root for a sample):

```toml
[package]
name = "myapp"
version = "0.1.0"
entry = "src/main.ryx"

[build]
runtime = "portable"   # Windows default; Linux may use "uring"
optimize = true
```

`rynixc build` / `run` pick up `[build]` when a `rynix.toml` is present in the working tree.

### Verify (CI-equivalent)

```sh
cargo clippy -p rynixc -p rynix-rir -p rynix-codegen -p rynix-sema -- -D warnings
rynixc arch check --error-format=json
python benchmarks/suite5/run_suite5.py --langs c,rynix
cd editors/vscode && npm ci && npm run compile
```

### FAQ (quick fixes)

| Problem | Fix |
| ------- | --- |
| **`clang not found`** (build/run) | Install system `clang` (Linux/macOS) or MinGW `x86_64-w64-mingw32-clang` on PATH (Windows). `check` / `fmt` / MCP work without clang. |
| **Windows link errors** | Pass `--runtime=portable`; target `x86_64-pc-windows-gnu`. See [INSTALL.md](INSTALL.md). |
| **Zig column missing in Suite5** | Zig is optional — install `zig` on PATH or run `--langs c,rust,go,rynix` / `--langs c,rynix` (CI subset). |
| **Checksum mismatch C ↔ Rynix** | Do not change workload constants; run `cargo test -p rynixc --test phase10_gates`. |
| **Slow or odd benchmark ms** | Re-run `python benchmarks/suite5/run_suite5.py --summary`; ratios vary by machine (±5–15%). |

---

## Language snapshot

```ryx
def main() -> i64
  let v: Vec[i64] = vec_new(0)
  v.push(1)
  v.push(2)
  let ok = true and v.len() == 2
  match ok
    true
      return v.get(0) + v.get(1)
    false
      return 0
  end
  return -1
end
```

### Examples


| File                                              | Demonstrates                |
| ------------------------------------------------- | --------------------------- |
| `[01_hello.ryx](examples/01_hello.ryx)`           | `print`                     |
| `[02_match_loop.ryx](examples/02_match_loop.ryx)` | `match` + `loop`            |
| `[03_vec.ryx](examples/03_vec.ryx)`               | `Vec[i64]` methods          |
| `[04_bool_logic.ryx](examples/04_bool_logic.ryx)` | `and` / `or`                |
| `[05_http_json.ryx](examples/05_http_json.ryx)`   | `json_get_i64` (no network) |


### Soft builtins (v0.1)


| Area        | API                                                                            | Limits                                                                                    |
| ----------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| I/O         | `print`, `print_i64`                                                           | Host write                                                                                |
| Fibers      | `spawn` (stmt), `yield`, `sleep_ms`, `now_ms`, `fiber_run`                     | Colorless concurrency; PARKED in `rt/` ([SPEC](docs/SPEC.md))                             |
| Collections | `vec_new`, `map_new`, methods                                                  | Mono `Vec[i64]` / `Map[i64,i64]` ([ADR-0006](docs/adr/0006-monomorphized-collections.md)) |
| TCP         | `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_recv`, `tcp_send`, `tcp_close` | Fiber-safe `rt/` loops                                                                    |
| JSON / HTTP | `json_get_i64`, `http_get_json_i64`                                            | Int fields only; CI tests json + connect-fail                                             |
| Reserved    | `tensor`, `signal`, `agent`                                                    | Keywords — stubs, not full std                                                            |


Grammar: `[docs/SPEC.md](docs/SPEC.md)`

---

## Architecture guard

`[Architecture.toml](Architecture.toml)` defines layout dirs and layer invariants.

```text
  Architecture.toml
        │
        ▼
  rynixc arch check ──▶ scan all .ryx under root
        │
        ├── layout: required dirs exist?
        └── invariants: forbidden imports / calls per glob
                │
                ▼
        pass │ violation_detected
                │
                └── --error-format=json → rynix.arch.v1
```

```sh
rynixc arch check
rynixc arch check --error-format=json
```

Schema: `[docs/schemas/rynix.arch.v1.json](docs/schemas/rynix.arch.v1.json)`

---

## Benchmarks

Full index: `[benchmarks/README.md](benchmarks/README.md)`

### Suite5 — 12 workloads × 5 languages (+ End optional)

Same **integer algorithms** in **Rynix · C · Rust · Go · Zig** (End when `endc` + `.end` ports exist).
Each binary prints one checksum; CI requires **C ↔ Rynix match on all 12** — **correctness
first**, not crown-speed marketing.

```sh
python benchmarks/suite5/run_suite5.py --summary                    # 5 langs + matrix
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end
python benchmarks/suite5/analyze_results.py                         # rank & gap-to-fastest
python benchmarks/suite5/run_suite5.py --langs c,rynix              # CI subset (faster)
```

**Methodology:** warmup=3, runs=9, reported ms = **trimmed median** (drops min/max when runs≥5).
Override via `SUITE5_WARMUP` / `SUITE5_RUNS` or `--warmup` / `--runs`.
Schema: `rynix.suite5.v2` in [`suite5_results.json`](benchmarks/suite5/suite5_results.json).

Sample run (Windows, 2026-08-22; **re-run on your machine** — numbers vary ±5–15%):

**All 12 C ↔ Rynix checksums pass** (`phase10_gates` + CI `suite5-check`).

| Workload | C   | Rust | Go  | Zig | Rynix  | Rynix/C   | Notes        |
| -------- | --- | ---- | --- | --- | ------ | --------- | ------------ |
| alu      | 7.6 | 7.5  | 10.1| 8.1 | **7.3**| **0.96×** | leads vs C   |
| nested   | 6.2 | **5.3** | 8.7 | 6.6 | 6.8 | 1.10×     | near parity  |
| fib      | 7.6 | **7.2** | 9.6 | 7.8 | 7.5 | 0.98×     | near parity  |
| hash     | 19.5| 17.8 | 18.8| 18.6| **15.7**| **0.81×**| lazy-or urem |
| prime    | 11.2| **10.1** | 15.6 | 10.9 | 11.0 | 0.99×  | near parity  |
| sum      | 6.1 | **5.6** | 8.6 | 6.2 | 6.8 | 1.12×     | near parity  |
| bits     | 451 | 437  | 436 | 484 | **90** | **0.20×** | `@llvm.ctpop` |
| matrix   | 8.5 | 7.4  | 65.8| 8.8 | **6.9**| **0.81×** | unrolled inner |
| scan     | **8.4** | 15.0 | 13.8 | 17.0 | 9.0 | 1.08×  | short-circuit `or` |
| powmod   | 16.1| **15.1** | 17.6 | 16.8 | 15.9 | 0.99×  | near parity  |
| gcd      | 167 | 167  | 219 | 168 | 167    | 1.00×     | near parity  |
| reduce   | 11.0| 15.7 | 20.2| 13.7| **10.9**| **0.99×** | LLVM 4-wide SIMD |

Times are rounded trimmed medians from the committed JSON (not marketing). Refresh locally:

```sh
python benchmarks/suite5/run_suite5.py --summary
```

Details: [`benchmarks/suite5/README.md`](benchmarks/suite5/README.md)

#### Performance honesty

Rynix is **not** at a performance ceiling. On this run:

- **Leads or ties C:** `bits`, `matrix`, `hash`, `scan`, `fib`, `alu`, `reduce`, several others within ~±10%.
- **Varies by machine/run:** re-run locally before drawing conclusions (`python benchmarks/suite5/run_suite5.py --summary`).
- **Not a merge gate:** optional LLVM PGO (`run_pgo_suite.py`) — deltas are machine-local and may regress.

**Compiler wins (in-tree tests):** counted-loop SSA, `urem` for nonneg paths (incl. literal divisors),
short-circuit `or`/`and` in conditional increments, `@llvm.ctpop`, `×31→lshl`, gcd inline + urem,
short-circuit `and`/`or` in `if`, loop `break` exit phis, `i*j+i` → `i*(j+1)` fold,
guarded outer loop (simple nested counted exits), merged guarded-loop exit blocks,
loop-invariant `iconst` hoist, LLVM loop vectorizer hints on latch back-edges,
small literal-trip loop unroll (≤8), nonneg symbol tracking for hot `urem`/`lshr` (scalar loops only — avoid extra loop-carried state that blocks LLVM vectorization).

### vs End [suite12](https://github.com/IrMaho/End/tree/main/benchmarks/suite12)


|       | End                                       | Rynix                                      |
| ----- | ----------------------------------------- | ------------------------------------------ |
| Shape | 12 challenges × 5 langs                   | **12 workloads × 5 langs** (Suite5)        |
| Style | Heavy sims (SDF, HFT, SHA-256, N-body, …) | Integer microkernels + checksum CI         |
| Stats | 5 runs + warmup                           | `--summary` matrix; optional local re-runs |


Different algorithms — **not row-comparable**. Rynix matches the **honest multi-lang**
shape; End still leads on spectacle benches. Mapping: `[benchmarks/README.md](benchmarks/README.md)`.

### Other harnesses


| Harness                       | Reference                                                                    |
| ----------------------------- | ---------------------------------------------------------------------------- |
| TCP echo RPS (fibers)         | `[docs/bakeoff.md](docs/bakeoff.md)` + optional `scripts/bakeoff_go_echo.go` |
| Lexer throughput (~400 MiB/s) | `[docs/benchmarks.md](docs/benchmarks.md)`                                   |
| Hello binary size             | `hello_binary_under_300kb` in `size_echo_gates`                              |


```text
  Suite5 (CPU micro)          Bakeoff (I/O)
  ─────────────────           ─────────────
  checksum gate first         fiber TCP echo RPS
  5 langs / 12 workloads      optional Go peer
         │                           │
         └──────── benchmarks/README.md ────────┘
```

---

## Acceptance gates

Every ✅ in `[docs/ROADMAP.md](docs/ROADMAP.md)` maps to a test or CI job.


| Gate                       | Evidence                                              |
| -------------------------- | ----------------------------------------------------- |
| Hello under 300 KiB        | `size_echo_gates` (skipped if clang absent)           |
| Fiber / TCP / load / uring | `rt/tests/`                                           |
| JSON unit + smoke          | `json_unit.c`, `json_smoke.c`                         |
| HTTP connect-fail          | `http_smoke.c`                                        |
| LLVM ↔ interpreter         | `diff_llvm_vs_interp`                                 |
| Phase 10 surface           | `phase10_gates` (arch, Suite5×12, http LLVM, VS Code) |
| RIR lowering patterns      | `gcd_urem`, `matrix_unroll`, `reduce_nonneg`, `scan_hash_lower`, … |
| LSP hover + goto-def       | `lsp_cmd` unit tests                                  |
| AI CLI JSON                | `agent_cli`                                           |
| ASan runtime               | CI Ubuntu sanitizer job                               |
| Clippy `-D warnings`       | CI clippy job                                         |


---

## Editor (VS Code)

```sh
cd editors/vscode && npm install && npm run compile
# or: .\editors\vscode\install_extension.ps1
```

```text
  .ryx file ──▶ VS Code extension ──▶ rynixc lsp-serve (stdio)
                         │
                         ├── diagnostics (check pipeline)
                         ├── hover (types)
                         └── go-to-definition
```


| Setting              | Purpose                |
| -------------------- | ---------------------- |
| `rynix.compilerPath` | Path to `rynixc`       |
| `rynix.enableLsp`    | Enable language server |


**v0.1:** grammar, diag, hover, def. **Not yet:** CodeLens, studio ([ADR-0007](docs/adr/0007-deferred-ui-frameworks.md)).

---

## Tooling surface

### AI-native CLI


| Command                          | Schema            | Purpose                  |
| -------------------------------- | ----------------- | ------------------------ |
| `check --error-format=json`      | `rynix.diag.v1`   | Structured diagnostics   |
| `graph`                          | `rynix.graph.v1`  | Call graph + edges       |
| `impact --fn=name`               | `rynix.impact.v1` | Callers / callees        |
| `eval --json`                    | `rynix.eval.v1`   | Constant expression eval |
| `arch check --error-format=json` | `rynix.arch.v1`   | Layer violations         |
| `slice`                          | —                 | Human outline            |
| `patch --write`                  | —                 | Apply compiler fixes     |


```sh
rynixc graph examples/02_match_loop.ryx
rynixc impact examples/02_match_loop.ryx --fn=main
rynixc eval --json "10 + 5"
```

Schemas: `[docs/schemas/](docs/schemas/)` · Agent guide: `[AGENTS.md](AGENTS.md)`

### MCP server

Start: `rynixc mcp-serve` (stdio JSON-RPC).


| #   | Tool                  | Role                  |
| --- | --------------------- | --------------------- |
| 1   | `diagnostics`         | File diagnostics      |
| 2   | `rynix_check`         | Check pipeline        |
| 3   | `rynix_format`        | Format source         |
| 4   | `rynix_explain_alloc` | Escape / alloc sites  |
| 5   | `compile`             | Build IR / binary     |
| 6   | `ast_query`           | AST queries           |
| 7   | `apply_fix`           | Apply suggested fixes |
| 8   | `rynix_graph`         | Call graph JSON       |
| 9   | `rynix_impact`        | Impact analysis       |
| 10  | `rynix_eval`          | Eval expressions      |
| 11  | `rynix_arch`          | Architecture check    |


### Release

Tag `v*` → GitHub Release: Linux + Windows `rynixc`, per-artifact SHA256,
`SHA256SUMS.txt` (`[.github/workflows/release.yml](.github/workflows/release.yml)`).

---

## Repository layout

```text
Rynix/
├── crates/                 # span → lexer → ast → parser → sema → rir → codegen
│   └── rynixc/             # CLI, LSP, MCP, arch, agent commands
├── rt/                     # rynix_rt_* runtime (C)
│   ├── include/            # public ABI header
│   ├── src/                # fiber, net, json, http, uring, collections
│   └── tests/              # C smokes (ASan in CI)
├── examples/               # runnable .ryx
├── benchmarks/suite5/      # 12 cross-lang workloads + harness
├── editors/vscode/         # LSP client + TextMate grammar
├── std/                    # prelude notes (.ryx docs)
├── docs/                   # SPEC, ROADMAP, ABI, ADRs, JSON schemas
├── Architecture.toml       # layer invariants
├── install.ps1 / INSTALL.sh
└── AGENTS.md               # guide for AI agents
```

---

## CI & quality

```text
  push / PR
     ├── test (Ubuntu + Windows)     cargo test --workspace (incl. phase10_gates)
     ├── clippy                      -D warnings
     ├── suite5-check                C ↔ Rynix checksum (12 workloads)
     ├── arch-check                  Architecture.toml
     ├── vscode-extension            npm ci && compile
     └── sanitizer (Ubuntu)          ASan: fiber, TCP, json, http, load
           └── uring smokes (Linux)   SQE + TCP + load

  workflow_dispatch (optional)
     └── Suite5 PGO                    train + benchmark + artifact upload
```

Workflows: [`.github/workflows/ci.yml`](.github/workflows/ci.yml),
[`.github/workflows/suite5-pgo.yml`](.github/workflows/suite5-pgo.yml),
[`.github/workflows/release.yml`](.github/workflows/release.yml).

---

## Status


| Scope                 | Detail                                                                                        |
| --------------------- | --------------------------------------------------------------------------------------------- |
| **Shipping**          | Phases **0–10** — [`docs/ROADMAP.md`](docs/ROADMAP.md) (each ✅ has in-tree tests)            |
| **Deferred**          | C11 backend — [ADR-0008](docs/adr/0008-deferred-c11-backend.md)                               |
| **Out of scope v0.1** | UI, hot-reload, canvas — [ADR-0007](docs/adr/0007-deferred-ui-frameworks.md)                  |
| **Perf gaps (honest)**| no parametric `Vec[T]`; no GPG-signed releases; ms ratios vary by machine/run |
| **Not claimed**       | End-style suite12 sims (SDF/HFT/SHA), inkwell backend, crown-speed on all 12 workloads          |


---

## Documentation


| Document                                             | Contents                         |
| ---------------------------------------------------- | -------------------------------- |
| [`AGENTS.md`](AGENTS.md)                             | Guide for AI agents / MCP        |
| [`INSTALL.md`](INSTALL.md)                           | Install, verify, troubleshooting |
| [`docs/SPEC.md`](docs/SPEC.md)                       | Grammar & builtins               |
| [`docs/abi.md`](docs/abi.md)                         | Runtime symbols                  |
| [`docs/diagnostics.md`](docs/diagnostics.md)         | `RYX####` codes                  |
| [`docs/END_PEER_GAP.md`](docs/END_PEER_GAP.md)       | Honest End peer gap & P0–P3 backlog |
| [`docs/COMPARE.md`](docs/COMPARE.md)                 | Peer comparison (End, etc.)      |
| [`docs/ROADMAP.md`](docs/ROADMAP.md)                 | Phase gates & evidence           |
| [`benchmarks/suite5/README.md`](benchmarks/suite5/README.md) | Suite5 harness & PGO       |
| [`PRODUCTION_READINESS.md`](PRODUCTION_READINESS.md) | Subsystem matrix                 |
| [`SECURITY.md`](SECURITY.md)                         | Vulnerability reporting          |
| [`CONTRIBUTING.md`](CONTRIBUTING.md)                 | Contribution guide               |
| [`docs/adr/`](docs/adr/)                             | Architecture decisions           |


---



**Rynix v0.1.0** — built to be verified, not merely advertised. Re-run benchmarks and gates on your machine before trusting any ms ratio.

