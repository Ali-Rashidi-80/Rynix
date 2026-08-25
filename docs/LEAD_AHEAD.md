# Lead ahead — Phase 12 (valuable, not theatrical)

**Status:** **Phase 12 complete** (all waves 0–6 + 1b green, 2026-08-25).
Phase 11 (A→E) is closed in-tree.
**Peer snapshot:** [IrMaho/End](https://github.com/IrMaho/End) `main` @ `cf5bef3`
(2026-08-24). TLS / Cranelift / registry remain simulated; ~62 `docs/*.md`; no MCP.

**Assurance (useful ≠ theatrical ≠ “globally fastest”):** see [§8](#8-assurance--useful-vs-speed)
+ live Suite5 matrix in [END_PEER_GAP.md](END_PEER_GAP.md) §2 /
[benchmarks/suite5/README.md](../benchmarks/suite5/README.md).

**Canonical execute checklist:** this file.
**Who-ahead audit (2026-08-25):** [VERDICT.md](VERDICT.md).
**Research archive (peer + web + agents, for future phases):**
[research/PHASE12_RESEARCH_INVENTORY.md](research/PHASE12_RESEARCH_INVENTORY.md).
**Cursor mirror (same locks):** `.cursor/plans/phase_12_lead_ahead_f7fee9e2.plan.md`
(Phase-12 mirror only; older roadmap plans are historical, not P12 locks).

Parent honesty: [SURPASS_END_PLAN.md](SURPASS_END_PLAN.md) §0,
[END_PEER_GAP.md](END_PEER_GAP.md), ADRs
[0007](adr/0007-deferred-ui-frameworks.md) /
[0008](adr/0008-deferred-c11-backend.md) /
[0009](adr/0009-agent-contracts-toolchain.md) /
[0010](adr/0010-local-package-index.md) /
[0011](adr/0011-suite12-divergent-benches.md).

**Rule:** a wave is done only when its **named gate** is green in-tree + SPEC/docs
match behavior. Prefer compiler fixes over loosening tests ([AGENTS.md](../AGENTS.md)).

---

## 0. North star

1. `rynixc new svc && cd svc && rynixc build` — no extra path args
2. Bounded multi-request JSON HTTP on the existing fiber/RT path
3. `import std::fs` (and thin crypto) without memorizing soft names
4. Suite12 MATCH kernels as `.ryx`, checksum-locked to C
5. MCP stays the agent surface; LSP jump works across workspace members
6. **README documents every real shipping domain** (evidence rows) — never End’s
   green wallpaper for stubs

Anything that does not move these needles is out of Phase 12.

---

## 0b. Why End’s README looks “richer” (and what we do instead)

End README feels complete because of **presentation**, not depth:

- Tagline “One Language. Every Domain” + 12-row domain table mostly 🟢 Stable
- Named frameworks (Hyper/Forge/Nexus/Crypto/KV) + EIP bullet laundry list
- ~62 docs / “10 pillars” hub as volume-as-capability
- Code samples that look like products (`@post`, `feature`/`skill`)

**Tree reality:** Hyper/Forge/KV/Raft/TLS/AES/GGUF/GPU/registry are thin or fake;
**no MCP**. Green ≠ shipping.

Rynix’s gap is the **opposite problem**: the architecture already ships real
HTTP/TLS/WS/crypto/KV/fs/MCP-18/packages/fibers/IOCP/uring/Suite5 — but README
**under-lists** them (soft table starved; CLI/MCP tables stop at ~11 tools;
`|>` / region / effects / `new` buried). That is a **docs honesty bug**, not a
missing architecture. Fix it in Wave 0.7 — do **not** copy End’s domain circus.

### Domain investment judgment (architecture fit + web research 2026-08-25)

Rynix stack = LLVM→clang · colorless fibers · portable/uring/IOCP · real TLS ·
NDJSON+MCP · path/local packages · escape/region/move · checksum benches.

**Key question:** does *this language* make that domain faster/better than existing
stacks, or does End-style README only create a green row?

| End-sold domain | Does Rynix make it better? | Phase 12 | Later re-entry (not theater) |
|-----------------|----------------------------|----------|------------------------------|
| Backend / HTTP APIs | **Yes** — fibers + real HTTP/TLS beat EndHyper stubs; async I/O is where Zig/Rust invest | **Wave 2** | deepen routing only with gates |
| WebSocket / framing | **Yes** — real WS already | Wave 0.7 docs | — |
| Memory-safe systems | **Yes** — escape/region/move | Wave 1b | — |
| Crypto / TLS | **Yes** — KATs + real TLS | 0.7 + Wave 4 | — |
| Packages (local) | **Yes** | Wave 1 | — |
| Agent tooling | **Yes** — industry 2025–26 = **MCP + Skills**, not 50 language keywords ([MCP vs Skills](https://www.developersdigest.tech/blog/mcp-vs-agent-skills)) | 0.7 + Wave 6 | Out of P12: Skills-as-docs packs; **never** End `feature`/`skill` keywords |
| Dev tools / small bins | **Yes** | 0.7 | — |
| Thin KV | **Yes** (narrow) | 0.7 docs | refuse “DB/WAL” marketing |
| Benchmarks | **Yes** | Wave 5 | — |
| Editor / LSP | **Yes** | Wave 6 | — |
| **Games / GPU / canvas** | **Not via language alone.** Indie engines in 2025 sit on **SDL3 / wgpu / ImGui** (asset/editor/physics dominate). Zero-GC helps *sim kernels*, not a 120 FPS product. End canvas = extern stubs. | **Refuse P12** | **Phase 14+** only with: real `wgpu`/SDL **FFI** + frame-time gate; never invent `std/ui` theater ([Noel Berry 2025](https://noelberry.ca/posts/making_games_in_2025/), [imgui](https://github.com/ocornut/imgui/)) |
| **Raft / consensus** | **Language ≠ Raft speed.** HashiCorp: bound by **disk + network**, not CPU language. Correctness needs years (etcd/raft, hashicorp/raft). End `raft.end` = structs. | **Refuse P12** | **Phase 15+** only as thin client to a proven lib, or research with Jepsen-class tests — never a Stable README row |
| **GGUF / LLM runtime** | **Kernels win, not syntax.** Production = llama.cpp / CUDA / Metal / pure-Rust engines with *tens of kSLOC* quant code. New-lang GGUF does not beat that; End GGUF = magic check. | **Refuse P12** | **Phase 15+** only as **FFI** to llama.cpp for agent-local models — never reimplement GGUF in `.ryx` |
| **Mobile (NDK/iOS)** | Cross-compile + JNI/UI is a **platform product** (Swift Android SDK took years of Apple work). Rynix does not improve app UX by existing. | **Refuse P12** | After native desktop/server solid; then `aarch64-linux-android` via clang if demand |
| **WASM** | Real portability/sandbox value; LLVM makes a **wasm32 emit** plausible. Still a full ABI+WASI surface. End WASM = text toy. | **Refuse P12** | **Phase 13 candidate** after Waves 1–4: `emit-wasm` via LLVM with one smoke gate — not “browser games without JS” |
| **CDN / PubGrub registry** | At *scale* CDN matters ([crates.io 2024 CDN](https://blog.rust-lang.org/2024/03/11/crates-io-download-changes/)); young langs start **path/git/local index** (Mochi/Cargo prior art). Fake PubGrub = End theater. | **Refuse P12** (ADR-0010) | After local packages + users: sparse index + Sigstore; CDN when download volume hurts |
| **50-contract language DSL** | Wrong layer. Agents need **MCP tools + evidence** (Rynix already) and Skills-as-docs — not `feature`/`task` keywords. | **Refuse forever as End syntax** (ADR-0009) | Deepen `verify`/contracts TOML + MCP |

**Architecture verdict:** Stack is **right** for systems + network + crypto + agent
toolchain. Incomplete as *product UX* (Waves 0–6). Domains above are not “never
improve performance” — they are **wrong next bets**: End-style stubs waste months
without beating C++/Rust/Go ecosystems where bottlenecks are engines, I/O, or GPU
kernels. Re-entry only with the gates in the table.

**Web research also affirms Phase 12 core:** invest in fiber/HTTP/async I/O honesty
(Zig/Rust competition is exactly there), local packages before CDN, MCP before
in-language agent DSLs.

---

## 1. Filter (every PR)

**Ship** if: user/agent notices without marketing + End cannot match with a parse
test + STATUS badge + red→green gate in this repo.

**Refuse (Phase 12):** C11 emit · canvas/UI stubs · CDN · fake JIT · 50/80/100 syntax ·
Raft/GGUF theater · End suite12 ms · HTTP/2 theater · README rows without evidence.
See §0b for **later re-entry** criteria (L17).

---

## 2. Locked decisions (no re-open during execute)

| ID | Lock |
|----|------|
| L1 | **Order:** `0 → 1 → (1b ∥ 2) → 3 → 4 → (5 ∥ 6)`. Do not start 3 until **both** 1b and 2 are green. Do not start 4 until 3 is green. |
| L2 | **Manifest resolve:** no `--manifest-path`. Accept: omitted path, directory, or `rynix.toml`. Resolve via `find_manifest` from cwd (or given dir). Direct `.ryx` path unchanged. |
| L3 | **Primary sources:** root package compiles **`[package].entry` then `files`** as one primary unit (same concat pattern deps already use in `codegen_pipe`). Deps via existing `CompileUnit.paths`. |
| L4 | **Runtime precedence:** CLI `--runtime` wins when the flag is **present**; else `[build].runtime`; else portable. Wave 1 uses `Option<RuntimeKind>` (or equivalent “flag seen” bit) — do not treat default Portable as an explicit CLI override. |
| L5 | **Optimize:** `[build].optimize` + CLI `--opt`/`--no-opt` wired in Phase 13 (P13-L5). Default for `build`/`run` remains optimize-on when unset. |
| L6 | **Field assign:** Wave 0 = **hard error** (`RYX2020` field/index assign unsupported). Wave 3 = implement store + flip fixtures from fail→pass. Register in `rynix-diag` `code.rs` + `docs/diagnostics.md`. |
| L7 | **Eval:** Wave 0 = disclosure in **README Tooling** (one paragraph) + **AGENTS.md** one line: `eval` is arith/print only; unsupported CallExt is undefined until Wave 6. Wave 6 = `interp` **hard-fail** on unsupported `CallExt` (no zero-default). **No** `docs/USAGE` file. |
| L8 | **HTTP:** `rynix_rt_http_serve_loop_json_i64(port, path, value, max_reqs)`; `max_reqs <= 0` → `-1`; on success after serving **exactly** `max_reqs` GETs matching `path`, return **`0`**; keep one-shot builtins (including `max_reqs == 1` may differ from serve_once internals — both must work). |
| L9 | **Struct v1:** `Name { field: i64, ... }` only; enum *values* deferred (one SPEC sentence). |
| L10 | **Wave 4 crypto:** SHA thin wrapper only (no HMAC facade this phase). |
| L11 | **Suite12:** ship `.ryx` for **#12 ALU, #4 SHA, #8 JSON** only; #1/#5/#6 stay [ADR-0011](adr/0011-suite12-divergent-benches.md). Gate names: `suite12_alu_ryx_checksum`, `suite12_sha256_ryx_checksum`, `suite12_json_ryx_checksum` in `size_echo_gates.rs`. |
| L12 | **LSP workspace:** resolve defs from **on-disk** workspace member sources via manifest (not only open buffers). |
| L13 | **Soft stubs:** Wave 0 = SPEC/README **reserved** **and** sema **rejects** calls to `tensor`/`signal`/`agent` with **`RYX2013`** (`STUB_RESERVED`). No RT. Wave 0.6 is not docs-only. |
| L14 | **No more peer deep-dives** required before execute; End@cf5bef3 facts locked. Re-open only if peer HEAD ≠ `cf5bef3` or Phase 12 exit. |
| L15 | **README richness = document truth**, not invent domains. Wave 0.7 expands soft/CLI/MCP/language/packages to match the tree; each later wave updates its README row when its gate turns green. |
| L16 | **No End-style “every domain 🟢” wallpaper.** Domain table stays evidence-gated; deferred rows only cite ADRs. |
| L17 | **Deferred ≠ worthless.** Games/GPU, Raft, GGUF, mobile, WASM, CDN stay **out of Phase 12** for reasons in §0b (bottleneck ≠ language / ecosystem years). Re-enter only with §0b gates (FFI+smoke, Jepsen, llama.cpp FFI, llvm wasm32, sparse index). Never ship End-style stubs to “cover” the README. |
| L18 | **Web + peer research for P12 closed (2026-08-25).** Inventory: [research/PHASE12_RESEARCH_INVENTORY.md](research/PHASE12_RESEARCH_INVENTORY.md). No further net/subagent sweeps required before execute unless a wave is blocked or End HEAD ≠ `cf5bef3`. |

---

## 3. Waves

### Wave 0 — Honesty freeze + P0 sema + README truth

| Step | Work | Gate |
|------|------|------|
| 0.1 | Record `git rev-parse HEAD` + `agent_cli` test count in §4 Tracking note | note filled |
| 0.2 | `END_PEER_GAP.md`: peer snapshot block End@cf5bef3 (TLS/Cranelift/registry stub; ~62 docs; no MCP) | paragraph present |
| 0.3 | SPEC: remove/qualify §2.6 struct-literal claim until Wave 3; align §5 soft table with `check.rs` (tls/ws/hmac/aes/fs) | SPEC consistent |
| 0.4 | README: stop claiming `[build]` is applied until Wave 1 wires `runtime` (L4/L5) | README consistent |
| 0.5 | Sema: field/index assign → diagnostic **`RYX2020`** (not silent no-op); register in `code.rs` + `diagnostics.md` | `sema_unit`: `field_assign_rejected` |
| 0.6 | Soft `tensor`/`signal`/`agent`: SPEC/README reserved **and** sema reject calls with **`RYX2013`**; eval disclosure in README Tooling + AGENTS.md (L7) | `sema_unit`: `stub_reserved_rejected` + README/AGENTS paragraphs |
| 0.7 | **README richness (L15):** expand Soft builtins to match `check.rs` groups (TCP/HTTP one-shot/JSON/frame/TLS/WS/crypto/KV/fs); full `rynixc` + MCP **18** tools (verify…dna); language teaser `|>` / `region` / `#^ effect: pure` / linear move; Packages subsection (unity/`pkg__fn`/lock/workspace/`new`); cite measured binary KiB from END_PEER_GAP; keep deferred rows ADR-only — **no new fake domains**. Pass condition: every new/changed maturity or soft-table row cites a path under `crates/`, `rt/`, `testdata/`, or `docs/schemas/` | same checklist in Wave 0 commit message |

**Primary files:** `docs/SPEC.md`, `README.md`, `AGENTS.md`, `docs/END_PEER_GAP.md`,
this file, `docs/diagnostics.md`, `crates/rynix-diag/src/code.rs`,
`crates/rynix-sema/src/check.rs`, `crates/rynix-sema/src/errors.rs`,
`crates/rynix-sema/tests/sema_unit.rs`.

**Per-wave README rule:** when Waves 1–6 gates turn green, update the matching
README row in the same commit (manifest UX, HTTP loop, struct, std import,
suite12 `.ryx`, workspace LSP).

---

### Wave 1 — Project as the unit of work

| Step | Work | Gate (home) |
|------|------|-------------|
| 1.1 | `BuildOptions.path: Option<PathBuf>`; `runtime: Option<RuntimeKind>` (L4); resolve → entry+`files` as primary paths | `agent_cli`: `build_from_manifest_entry` |
| 1.2 | `codegen_pipe`: change primary to multi-path concat like deps (`&[PathBuf]` or primary `CompileUnit`) | covered by 1.1 |
| 1.3 | Apply `[build].runtime` when CLI runtime flag absent (L4) | assert in `build_from_manifest_entry` (manifest `runtime = "iocp"` or portable on Win) |
| 1.4 | `rynixc run` same resolve | `run_from_manifest_entry` |
| 1.5 | `new`: print `next: rynixc build`; scaffold builds with no path | extend `new_scaffolds_package` |
| 1.6 | Missing `entry` → JSON diag, not panic | `agent_cli`: `build_missing_entry_diag` |
| 1.7 | README quick start + AGENTS.md one line | docs |

**Primary files:** `cli.rs`, `build_cmd.rs`, `run_cmd.rs`, `manifest.rs`,
`codegen_pipe.rs`, `new_cmd.rs`, `crates/rynixc/tests/agent_cli.rs`,
fixture **`testdata/pkg_app`** (exists; use it).

---

### Wave 1b — Negative memory corpus *(parallel with Wave 2 after Wave 1)*

| Step | Work | Gate |
|------|------|------|
| 1b.1 | `testdata/compile_fail/memory/` **exactly 3** fixtures using **existing** codes only: (1) use-after-move → `RYX2011`, (2) pure violation → `RYX2012`, (3) reserved stub call → `RYX2013` | files exist |
| 1b.2 | Assert **codes** (not full message text) | codes asserted |
| 1b.3 | Runner | `agent_cli`: `compile_fail_memory_corpus` |

**Out of scope for 1b:** inventing `RYX3xxx` escape-reject codes. Escape/placement stays
positive via `--explain-alloc` / existing region tests (research inventory §3.3).

**Primary files:** `testdata/compile_fail/memory/*.ryx`; `crates/rynixc/tests/agent_cli.rs`.

---

### Wave 2 — Bounded HTTP loop *(parallel with Wave 1b after Wave 1)*

| Step | Work | Gate |
|------|------|------|
| 2.1 | RT C: `rynix_rt_http_serve_loop_json_i64` + `max_reqs` (L8) | C smoke in `rt/tests/` |
| 2.2 | Soft + sema + effects + lower + LLVM decls | `check`/`build` `.ryx` |
| 2.3 | Integration: 3 sequential GETs then exit | `size_echo_gates`: `http_loop_three_gets` |
| 2.4 | `docs/abi.md` + keep one-shots | docs |

**Primary files:** `rt/src/http.c` (or sibling), `rt/include/rynix_rt.h`,
`check.rs`, `effects.rs`, `lower.rs`, `emit.rs`, `docs/abi.md`,
`crates/rynixc/tests/size_echo_gates.rs`.

---

### Wave 3 — Struct values (i64 v1)

| Step | Work | Gate |
|------|------|------|
| 3.1 | SPEC grammar + i64-only + enum values deferred | SPEC |
| 3.2 | AST `StructLit` + parse + sema + RIR + LLVM | unit + build |
| 3.3 | Field **store** (clears RYX2020 for supported stores); flip Wave 0 fail fixtures | `struct_literal_field` |
| 3.4 | Gate home: `sema_unit` + `agent_cli` build/print field | both green |

**Primary files:** `docs/SPEC.md`, `rynix-ast`, parser, `check.rs`, `lower.rs`,
codegen, `sema_unit.rs`, `agent_cli.rs`.

---

### Wave 4 — `std` as loaded modules

| Step | Work | Gate |
|------|------|------|
| 4.1 | `std/fs.ryx` thin `def`s → soft `fs_*`; app uses `fs.write_file` | `build_fs_via_std_import` (`agent_cli`) |
| 4.2 | `std/crypto.ryx` thin SHA only (L10) | `build_crypto_sha_via_std` (`agent_cli`) |
| 4.3 | Docs-only modules stay without `def` | no change |

**Primary files:** `std/fs.ryx`, `std/crypto.ryx`, testdata app, `agent_cli.rs`.

---

### Wave 5 — Suite12 MATCH as `.ryx` *(parallel with Wave 6 after Wave 4)*

| Step | Work | Gate |
|------|------|------|
| 5.1 | Ports #12 / #4 / #8 | `suite12_*_ryx_checksum` in `size_echo_gates.rs` |
| 5.2 | README note: not End ms table | `benchmarks/suite12/README.md` |

---

### Wave 6 — LSP + verify + eval hard-fail *(parallel with Wave 5 after Wave 4)*

| Step | Work | Gate |
|------|------|------|
| 6.1 | Go-to-def across workspace members on disk (L12) | `lsp_workspace_def` in `lsp_cmd.rs` tests |
| 6.2 | `verify` contract evidence for manifest build | `agent_cli`: `verify_manifest_build_evidence`; contract under `docs/contracts/wave12_manifest.contract.toml` |
| 6.3 | `interp` hard-fail unsupported CallExt (L7) | `interp` unit: `eval_call_ext_hard_fail` |
| 6.4 | No `skill`/`task`/`feature` keywords | ADR-0009 |

---

## 4. Tracking

| Wave | Theme | Gate test | Status |
|------|--------|-----------|--------|
| 0 | Honesty + RYX2020/2013 + README truth | `field_assign_rejected` + `stub_reserved_rejected` + peer note + 0.7 | ✅ |
| 1 | Manifest build/run | `build_from_manifest_entry` | ✅ |
| 1b | Memory compile_fail | `compile_fail_memory_corpus` | ✅ |
| 2 | HTTP loop | `http_loop_three_gets` | ✅ |
| 3 | Struct i64 + store | `struct_literal_field` | ✅ |
| 4 | std fs/crypto | `build_fs_via_std_import` + `build_crypto_sha_via_std` | ✅ |
| 5 | Suite12 `.ryx` | `suite12_alu_ryx_checksum` (+sha/json) | ✅ |
| 6 | LSP / eval | `lsp_workspace_def` (+ `verify_manifest_build_evidence`, `eval_call_ext_hard_fail`) | ✅ |

**Freeze note (fill on Wave 0.1):** HEAD=`461c6fd5c60730229162f86dd9cdd76bcf5ff9be` · agent_cli tests=`31`

**Atomic pre-execute scan (2026-08-25):** holes closed — ADR-0011 link, L7 disclosure home,
L13=RYX2013 reject, Wave 1b codes, Wave 0 `code.rs`/`diagnostics.md`, L4/L8/Wave 6 named gates.
**Verdict: READY_TO_EXECUTE.** Wave 0 execute in progress / gates below.

---

## 5. Definition of done (Phase 12 exit)

Binary checklist:

1. Waves **0–4 and 1b** green (5–6 may trail).
2. `new` → `build` with no hidden path.
3. SPEC/README/`[build].runtime` consistent with driver (L4/L5).
4. README Soft/CLI/MCP/language/packages match the tree (L15); no End wallpaper (L16).
5. `END_PEER_GAP` peer = End@cf5bef3 + new gate names listed under “Where Rynix leads”.
6. Zero new UI/C11/CDN/JIT stubs.

**Commits:** one commit per wave; message on *why*.

---

## 6. Calendar (sequence, not a promise)

| Span | Wave |
|------|------|
| Day 0 | Wave 0 |
| Days 1–2 | Wave 1 |
| Days 3–4 | Wave 1b ∥ Wave 2 |
| Days 5–8 | Wave 3 |
| Days 9–10 | Wave 4 |
| Days 11–14 | Wave 5 ∥ Wave 6 |

---

## 7. First command when executing

**Wave 0** (honesty + `RYX2020` + peer snapshot + **README truth 0.7**), then
immediately **Wave 1.1** (`build` without source arg). Do not warm up with
canvas, C11, or extra keywords. Do not invent README domains End markets but
stubs.

**Peer re-audit / net research:** not required before execute (L14, L18). Re-check End
only at Phase 12 exit or if HEAD moves past `cf5bef3`. Re-open deferred domains only
via L17 gates.

---

## 8. Assurance — useful vs speed

Phase 12 closed **product realness** gaps (manifest UX, HTTP loop, structs, `std`,
Suite12 `.ryx` checksum, LSP/verify). It did **not** invent a new microkernel optimizer
for every wave. Perf leadership vs peers stays where evidence already lived: **Suite5**
(checksum + disclosed strength reduction), **binary-size gates**, **real TLS/crypto**
vs End stubs — see [END_PEER_GAP.md](END_PEER_GAP.md) / [COMPARE.md](COMPARE.md).

### 8.1 Wave deliverables — useful now? theatrical? max-perf?

| Wave | Shipping surface | Useful *today*? | Theatrical? | “Max perf” claim? |
|------|------------------|-----------------|-------------|-------------------|
| 0 | Honesty + RYX2020/2013 + README truth | **Yes** — stops fake ✅ / stub keywords | Anti-theater | N/A (docs + reject) |
| 1 | `new` → `build`/`run` from `rynix.toml` | **Yes** — required package UX | No | N/A (driver UX) |
| 1b | compile_fail RYX2011/2012/2013 | **Yes** — memory/effect honesty | No | N/A (negatives) |
| 2 | `http_serve_loop_json_i64` | **Yes** — multi-GET on fiber/RT path | No (real bind/serve) | **No** — same RT as one-shot; not nginx/RPS bakeoff |
| 3 | i64 `StructLit` + field store | **Yes** — language completeness | No | **No** — same LLVM scalar path as locals |
| 4 | `import std::fs` / `std::crypto` (SHA) | **Yes** — soft names → import ergonomics | No | **No** — wrappers over existing RT |
| 5 | Suite12 `#12/#4/#8` as `.ryx` | **Yes** — checksum = C MATCH ports | No | **Correctness**, not End suite12 ms tables |
| 6 | LSP workspace def + verify evidence + CallExt hard-fail | **Yes** — agent/editor gates | No | N/A (tooling) |

**Not built as empty scaffolding:** every row above has a **named gate** in §4.
Deferred domains (games/Raft/GGUF/CDN/WASM) stay out — see §0b — so we did **not**
ship End-style green wallpaper for “coverage.”

### 8.2 Where performance *is* maximized (and how we know)

| Surface | What we claim | What we refuse | Evidence |
|---------|---------------|----------------|----------|
| Suite5 vs C/Rust/Go/Zig/(End) | Same checksum; wall-clock of finished binary; Rynix often leads after **disclosed** strength reduction | “Identical instruction work” across langs; Suite5 ms = End suite12 ms | CI `suite5-check`; local `--summary`; [suite5/README.md](../benchmarks/suite5/README.md) |
| Hello / bench binary size | Full-RT hello **&lt;300 KiB** gate; `--bench` slim RT | Inflating KiB without the gate | `hello_binary_under_300kb` |
| HTTP / TLS / WS | **Real** I/O (not plaintext-as-TLS) | Fake RPS vs Hyper stubs | `size_echo_gates`, RT tests |
| Suite12 `.ryx` | Checksum lock to C MATCH ids | Peer marketing ms | `suite12_*_ryx_checksum` + ADR-0011 skips |

Refresh Suite5 (this machine, 2026-08-25): Rynix/C median ratios from **0.19** (`bits`)
to **0.98** (`sum`) — see END_PEER_GAP §2. **`endc` not on PATH** → End column skipped;
prior 12/12 vs End (2026-08-23) still stands until re-run with `endc`.

### 8.3 Bottom line for “are we sure?”

1. **Useful:** Phase 12 closes real agent/package/HTTP/language gaps End markets but
   often stubs — gates prove behavior, not README rows.
2. **Not theatrical:** no C11/UI/CDN/JIT/Raft/GGUF stubs; stub keywords rejected
   (`RYX2013`).
3. **Perf maximized where it belongs:** Suite5 + size + real crypto/TLS — **not** by
   claiming Wave 2–4 are “faster than every language for every job.”
4. **Vs ready-made langs in-repo:** C/Rust/Go/Zig are Suite5 peers; Rynix wins many
   rows via LLVM + strength reduction (disclosed). That is competitive on *these*
   kernels, not a claim that Rynix replaces nginx or llama.cpp.
