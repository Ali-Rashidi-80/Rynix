# Phase 12 research inventory (peer + web + agents)

**Status:** closed for Phase 12 execute readiness (2026-08-25).  
**Atomic scan:** holes found then closed in LEAD_AHEAD (ADR-0011 link, L7/L13,
Wave 1b codes, Wave 0 diag registry files, L4/L8/Wave 6 gate names).  
**Verdict:** **READY_TO_EXECUTE** — see LEAD_AHEAD §4.  
**Execute order:** still [LEAD_AHEAD.md](../LEAD_AHEAD.md) (locks L1–L18).  
**Honesty:** this file cites *sources and conclusions*; it does not grant README ✅
without in-tree tests ([AGENTS.md](../../AGENTS.md)).

---

## 1. Peer End snapshot

| Field | Value |
|-------|--------|
| Clone | `D:\0\End-peer` (IrMaho/End) |
| HEAD | `cf5bef3a66d8c21863bed205c352319fafd580a4` |
| Date | 2026-08-24 |
| Docs | ~62 `docs/*.md` |
| MCP | **none** |
| Real | C11→zig/gcc/clang; agent JSON CLI; TCP; SHA/HMAC; thin Hyper; suite12 harness; partial regions |
| Fake/stub | TLS plaintext+cipher label; Cranelift without crate + fake addr; WASM text; PubGrub/registry synthetic; fibers theater; SMT w/o Z3; VS Code without LanguageClient; many 🟢 Stable vs ROADMAP Planned |

**Subagent campaigns (this chat arc):** End docs×code inventory; Rynix claim validation;
plan internal-defect audit; End README domain map (~35–39 sold surfaces); Rynix README
under-documentation audit; build-ready lock sync.

**Do not re-clone for P12 execute** unless HEAD ≠ `cf5bef3` (LEAD L14).

---

## 2. Why End README looks richer (conclusion)

Presentation tricks: “Every Domain” matrix, framework brand names, badge row,
EIP bullet laundry, 61-guide hub, suite12 crown table.  
**Not** shipping depth. Rynix inverse problem: real stack under-listed in README
→ Wave **0.7** (LEAD L15), not End wallpaper (L16).

---

## 3. Domain judgment + web sources (2026-08-25)

### 3.1 Invest in Phase 12 (architecture wins)

| Domain | Why | Sources / notes |
|--------|-----|-----------------|
| HTTP / fiber I/O | Async I/O is where Zig/Rust compete; Rynix already has fibers+uring/IOCP+HTTP one-shot | [corrode async Rust](https://corrode.dev/blog/async/), Zig/Tokio TCP comparisons 2025–26 |
| TLS / crypto KAT | Real SChannel/OpenSSL + NIST paths beat End fake TLS | Peer tree vs Rynix `size_echo_gates` |
| Local packages | Young langs start path/git/local; CDN later | [Mochi registry prior art](https://mochi-lang.dev/docs/research/0057/prior-art-registries), ADR-0010 |
| Agent MCP | Industry 2025–26: MCP tools + Skills — not 50 language keywords | [MCP vs Skills](https://www.developersdigest.tech/blog/mcp-vs-agent-skills), Red Hat / Anthropic Skills notes |
| Memory diagnostics | Negative corpora (`compile_fail` / trybuild) are maturity signals | [Rust compile_fail / trybuild](https://users.rust-lang.org/t/checking-which-compile-time-errors-happen/86859) → Wave **1b** = RYX2011/2012/2013 only (no RYX3xxx in P12) |
| Struct values | Named struct lits are baseline for systems langs + agent codegen | [Rust Reference struct expr](https://doc.rust-lang.org/reference/expressions/struct-expr.html) → Wave **3** |
| Suite12 MATCH `.ryx` | Checksum honesty > ms marketing | ADR-0011; peer footnotes |
| LSP workspace | Real LanguageClient already; deepen cross-file | Peer VS Code not wired |

### 3.2 Defer (not worthless) — re-entry gates

| Domain | Bottleneck ≠ Rynix syntax | Re-entry (future phase) | Key sources |
|--------|---------------------------|-------------------------|-------------|
| Games / GPU / canvas | SDL3 / wgpu / ImGui + assets/editor | wgpu/SDL **FFI** + frame-time gate; never stub `std/ui` | [Noel Berry 2025](https://noelberry.ca/posts/making_games_in_2025/), [Dear ImGui](https://github.com/ocornut/imgui/), HN game-without-engine 2025 |
| Raft | Disk + network; years of correctness | Thin client to etcd/hashicorp raft + hard tests | [hashicorp/raft](https://github.com/hashicorp/raft/) (“bound by disk I/O and network latency”), [etcd/raft](https://github.com/etcd-io/raft) |
| GGUF / LLM runtime | Quant/GPU kernels (10k–100k SLoC) | **FFI** to llama.cpp / mature Rust engine — not `.ryx` rewrite | oxillama / airframe / llama.cpp ecosystem 2025–26 |
| Mobile | NDK / iOS / JNI product | After native core; clang android target if demand | Swift Android SDK years-long effort notes 2026 |
| WASM | Useful; LLVM helps | **Phase 13 candidate:** llvm `wasm32` + one smoke (WASI later) | [WASI languages](https://wasi.dev/languages), Zig/Rust wasm targets; LLVM wasm triple docs |
| CDN / PubGrub registry | Needed at *scale* | After local adoption: sparse index + Sigstore; CDN when downloads hurt | [crates.io CDN 2024](https://blog.rust-lang.org/2024/03/11/crates-io-download-changes/), Mochi MEP-57 survey |
| 50-contract language DSL | Wrong layer | Deepen MCP/`verify` + optional Skills docs; **never** End `feature`/`skill` keywords | MCP vs Skills sources above; ADR-0009 |

### 3.3 Gap-fill searches (same day)

| Topic | Finding for Rynix |
|-------|-------------------|
| `compile_fail` corpora | Rust maturity pattern = fixtures + stable codes (trybuild-class). Affirms Wave **1b**. Prefer assert **codes** (`RYX####`) over fragile full messages. |
| Struct literals | Named fields are the agent-friendly / Rust-like form. Affirms Wave **3** `Name { x: i64 }` v1. |
| LLVM→WASM timing | Add after core product UX; llvm path is the cheap entry; WASI/component model is the long tail. Affirms Phase **13** candidate, not P12. |

---

## 4. Rynix tree facts locked for P12 (claim validation)

Confirmed in-tree (subagent + spot-check):

- `build` requires `.ryx` path; `[build].runtime` parsed unused (Wave 1)
- SPEC struct-literal claim vs no `StructLit` AST (Wave 0 + 3)
- Field assign silent no-op → `RYX2020` (Wave 0); store in Wave 3
- HTTP serve-once only (Wave 2 loop)
- `std/fs.ryx` / `std/crypto.ryx` docs-only (Wave 4)
- MCP 18 vs README table ~11 (Wave 0.7)
- LSP single-buffer go-to-def (Wave 6)
- Suite12: 9 C MATCH, no `.ryx` (Wave 5)
- `new` next-step still `check path` (Wave 1)

---

## 5. Phase map (research → work)

```text
P12 Waves 0–6     ← execute now (LEAD_AHEAD)
P13 candidate     ← llvm wasm32 smoke (if demand)
P14+              ← wgpu/SDL FFI if game/tooling need
P15+              ← llama.cpp FFI; Raft client only with hard tests
Post-adoption     ← sparse registry + Sigstore; CDN when scale hurts
Never as End DSL  ← feature/skill/task keywords (ADR-0009)
```

---

## 6. What is *not* needed before P12 execute

- Another End full-repo deep dive (L14)
- Another net sweep of deferred domains (L18) — recorded above
- Copying End’s 62-doc hub or green domain matrix

**Re-open research when:** End HEAD moves past `cf5bef3`; a P12 wave is technically
blocked; or starting P13+ re-entry and need fresh WASI/wgpu/registry state.

---

## 7. Cursor / chat artifacts

| Artifact | Role |
|----------|------|
| [LEAD_AHEAD.md](../LEAD_AHEAD.md) | SoT execute locks L1–L18 |
| `.cursor/plans/phase_12_lead_ahead_f7fee9e2.plan.md` | Checklist mirror |
| [END_PEER_GAP.md](../END_PEER_GAP.md) | Evidence table (peer snapshot on Wave 0.2) |
| [SURPASS_END_PLAN.md](../SURPASS_END_PLAN.md) | Parent honesty filter |
| This file | Research archive for future phases |

Agent transcripts for this arc live under the Cursor project `agent-transcripts/`
(session that produced Phase 12 locks); prefer this file over re-reading chat.
