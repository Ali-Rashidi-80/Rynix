# Surpass End — ordered development plan

Reference peer: local `D:\0\End-peer` ↔ [IrMaho/End](https://github.com/IrMaho/End)
(`endc` 2.0.0 crate / marketed as `0.4.0-alpha`).

This plan is derived from **reading End docs + surveying `endc` / `std` / frameworks**,
then filtering by code reality. It is the Phase 11 execution order for making Rynix
**more valuable** than End without copying End prose or marking ✅ without tests
([AGENTS.md](../AGENTS.md)).

Related: [END_PEER_GAP.md](END_PEER_GAP.md) · [ROADMAP.md](ROADMAP.md) ·
[ADR-0007](adr/0007-deferred-ui-frameworks.md) · [ADR-0008](adr/0008-deferred-c11-backend.md) ·
[ADR-0009](adr/0009-agent-contracts-toolchain.md).

---

## 0. Honesty filter (do this first, forever)

End’s docs claim a huge “Stable” universe. **Code inventory disagrees** on many rows.

| End claim | Reality in `End-peer` | Rynix response |
|-----------|----------------------|----------------|
| Cranelift JIT | Simulated; no cranelift crate; runs interpreter | **Skip** — keep real LLVM |
| Global package registry “Stable” | Staging / stub `lib.end` install | Build **real** local packages first |
| MCP server | **Absent** | **Already lead** (`rynixc mcp-serve`) |
| Native `spawn` / M:N fibers | C emit stub `/* spawn */` | **Already lead** (real fibers + uring tests) |
| suite12 “same algorithm” | Some checksums diverge across langs | Keep Suite5 checksum gate; optional suite12 later |
| 50 contracts / 80 DNA / 50 paradigms | Parser surface + marketing; depth uneven | Prefer **toolchain evidence** (ADR-0009), not keyword clones |
| LLVM AOT product | `.ll` text emit only; no LLVM link in cargo | **Already lead** (LLVM → binary + size gates) |
| FEATURES/STATUS green badges | Contradict ROADMAP / PACKAGES / AGENTS | Never mirror green badges without tests |

**Rule:** Only adopt End *capabilities that exist in working code or clear SPEC*.
Treat FEATURES.md / STATUS.md Stable tags as **untrusted** unless path + test cited.

### Where Rynix already leads (keep and advertise honestly)

1. Checksum-gated Suite5 CI (`c` ↔ `rynix`)
2. LLVM differential vs interpreter
3. Escape / `--explain-alloc` + MCP
4. Real fiber runtime + Linux io_uring tests
5. MCP protocol (End has CLI agent tools, not MCP)
6. Doc honesty culture (no fake ✅)

### Where End still looks broader (product surface)

1. Language extras: denser `std/` wrappers / `operation` naming (Rynix covers region/pipe/effects/move)
2. Optional agent `dna` heuristics (Rynix: verify/precheck/context/security/scope/deps shipped)
3. Frameworks narrative beyond HTTP/crypto/KV (UI canvas deferred ADR-0007)
4. suite12-class heavy sims (optional; Suite5 is the gated harness)
5. C11-first path (Rynix: LLVM + ADR-0008; size via `--bench`)

---

## Phase order (execute top → bottom)

Each phase ends only when **SPEC/ADR (if language) + tests + README honesty** land.

### Phase A — Language core that End actually parses & lowers

**Goal:** Close ergonomic gaps agents notice first, without End syntax clones for contracts.

| Step | Deliverable | Acceptance | Notes |
|------|-------------|------------|-------|
| A1 | Ephemeral resource scopes (`lease`/`during` *or* Rynix-native name) | ✅ `region … end` → RegionCreate/Reset + SPEC §3.1 | Prefer `region` over End `lease` clone |
| A2 | First-class pipeline / `operation`-like composition | ✅ SPEC §3.2 + `pipe_desugar` + `examples/09_pipe.ryx` | Prefer Rynix naming; algebra `>>` only if SPEC’d |
| A3 | Effects / capability annotations on fn (toolchain-checkable) | ✅ `#^ effect: pure` → `RYX2012` + `effects_pure` + verify contract evidence | OS sandbox later; static effect sets |
| A4 | Stronger borrow diagnostics (use-after-move, exclusive mut) | ✅ use-after-move `RYX2011` + sema tests; exclusive `&` conflict deferred (no surface refs in SPEC) | Match *behavior*, not End codes |
| A5 | Close Suite5 gaps `nested` + `powmod` vs End | ✅ opaque `nested` residue O(m²) loops; opaque `powmod` → binary `emit_modpow`; checksums match | Perf only after fairness |

**Out of scope for A:** morphic/quantum/bot “50 syntaxes”, DNA layers as language keywords.

---

### Phase B — Agent / EIP depth (toolchain, not keyword circus)

**Goal:** Match End’s *useful* agent CLI while staying MCP-first (ADR-0009).

| Step | Deliverable | Acceptance |
|------|-------------|------------|
| B1 | `rynixc verify --contract=…` | ✅ TOML → file/cargo_test evidence; schema + CLI + MCP |
| B2 | `precheck` / blast-radius (impact + write gate) | ✅ `rynix.precheck.v1` + CLI + MCP |
| B3 | `context` packer (slice → token budget) | ✅ `rynix.context.v1` + CLI + MCP |
| B4 | `security` AST scanner (subset of real CWEs) | ✅ `rynix.security.v1` + CLI/MCP; disclaimer not full audit |
| B5 | `scope` / permission profile for agent tools | ✅ deny-by-default `patch --write` unless scope/`--force-write` |
| B6 | Optional `dna` / conventions report | ✅ `rynixc dna` + `rynix.dna.v1` + MCP + `agent_cli` |
| B7 | Keep/extend MCP parity for every new CLI | ✅ verify/precheck/context/security/scope/deps/dna |

**Already have:** `graph`, `slice`, `impact`, `eval`, `patch`, `arch`, MCP, diag JSON,
`verify`, `precheck`, `context`, `security`, `scope`, `http_serve_once_json_i64` (C smoke + example).

**Do not clone:** in-language `feature`/`skill`/`task` keywords unless ADR-0009 superseded.

---

### Phase C — Stdlib & frameworks (real code > README domains)

**Goal:** Make domain table rows *earned*, one vertical at a time.

| Step | Deliverable | Acceptance | End analogue |
|------|-------------|------------|--------------|
| C1 | HTTP **server** (listen + route + respond) | ✅ `http_serve_once_json_i64` + C fiber smoke | EndHyper |
| C2 | JSON request/response helpers beyond GET smoke | ✅ `json_has_i64` + `http_post_json_i64` + echo serve + C smoke | Hyper DTO story |
| C3 | WebSocket or binary framing (minimal) | ✅ `frame_*` + RFC6455 `ws_*` frames/echo smoke | EndForge |
| C4 | Crypto: SHA-256 + HMAC + AES-GCM KAT | ✅ SHA/HMAC/AES-GCM NIST smoke (real AEAD; End AES is stub) | EndCrypto |
| C5 | Embedded KV (arena table + get/put) | ✅ `kv_new`/`put`/`get`/`len` smoke | EndKV |
| C6 | Soft `tensor` / signal stay soft until real ops | No “AI Stable” without ops | End tensors Experimental |

**Deferred (ADR-0007):** canvas / 120 FPS UI / hot-reload / game Nexus — after C1–C5.

---

### Phase D — Runtime & packaging depth

| Step | Deliverable | Acceptance |
|------|-------------|------------|
| D1 | TLS for TCP (SChannel Win; OpenSSL via `-DRYNIX_RT_OPENSSL`) | ✅ echo smoke `tls_echo_smoke_c` (real crypto; End “TLS” is simulated) |
| D2 | Better async I/O on Windows (IOCP or documented poll quality) | ✅ `--runtime=iocp` + `iocp_echo_smoke` (WSARecv/WSASend) |
| D3 | Local package story: `rynix.toml` **path** deps + `rynixc new` | ✅ deps/build gate + `new` scaffold + `testdata/pkg_*` |
| D4 | Lockfile + reproducible builds note | ✅ root `Cargo.lock` + CI `--locked`; see note below |
| D5 | Optional C11 emit (ADR-0008) only if LLVM path blocked somewhere | ✅ **deferred with rationale** — LLVM + C RT; ADR-0008 reaffirmed 2026-08-23 |

**Skip:** fake Cranelift JIT reports; stub registry packages.

**D4 note:** Rust builds are reproducible via the committed workspace
[`Cargo.lock`](../Cargo.lock). CI runs `cargo test --workspace --all-targets --locked`
so drift fails the job. Rynix path packages (`rynix.toml` deps) intentionally have
**no** separate lockfile yet — only local `{ path = … }` resolution (no registry).

---

### Phase E — Product polish & competitive benches

| Step | Deliverable | Acceptance |
|------|-------------|------------|
| E1 | VS Code CodeLens (diag / alloc / impact) | ✅ `editors/vscode` CodeLens + `npm run compile` |
| E2 | Optional suite12 *ports* or honest refuse | ✅ policy + ALU/HFT/JSON/FSM checksum gates |
| E3 | Binary size matrix automation in CI | ✅ `size-gate` job + `hello_binary_under_300kb` |
| E4 | Signed releases (beyond SHA256SUMS) | ✅ `scripts/build_release.ps1` + optional GPG in `release.yml` (secret-gated) |
| E5 | Install polish (`new` + INSTALL scripts) | ✅ `rynixc new` + INSTALL.md / install.ps1 |

---

## What *not* to build (End noise that would dilute Rynix)

1. Simulated backends that print fake JIT addresses
2. “Stable” framework READMEs wrapping empty stubs
3. Fifty revolutionary syntax catalogs without lowering/tests
4. Claiming suite12 wins without same checksums
5. Copying End’s version inflation (`v2.0.0` notes vs `0.4.0-alpha`)

Beating End on **value** means: **same or better real features + stricter evidence**.

---

## Suggested calendar (evidence-first, not calendar-forced)

| Wave | Phases | Outcome |
|------|--------|---------|
| Wave 1 | A1–A4, B1–B3 | Language + agent verify/precheck/context |
| Wave 2 | C1–C4, A5 | Real HTTP + crypto; Suite5 nested/powmod |
| Wave 3 | C5, D1–D3, B4–B5 | KV + TLS + packages + security/scope |
| Wave 4 | E1–E3, D4 | Editor + optional suite12 + size CI |

After Wave 2, re-score [END_PEER_GAP.md](END_PEER_GAP.md) §3 (“more valuable?”).

---

## Peer inventory snapshot (2026-08-23)

### End shipping core
- Single-crate `endc`: lexer → semantic → **C11 → gcc/clang/zig**
- Interpreter VM default for `run`
- Regions / partial borrow / large AST surface
- Agent CLI commands (graph/slice/…/dna/…) as Rust tools
- `std/*.end` wrappers (TCP, SHA-256, thin Hyper/Raft/…)
- VS Code extension + LSP skeleton
- suite12 harness + JSON results

### End partial / simulated
- LLVM `.ll` only; Cranelift fake JIT; WASM WAT
- `spawn` stub in C; OpenMP pragmas for some parallel
- Package install stubs; registry staging
- Capability engine in-memory, not OS sandbox

### End docs-only / unreliable maturity
- Global registry Stable; WASM Beta vs Planned; PRODUCTION_READY_V1 “complete”
- 3-tier vs 4-tier memory docs; 15 KB vs 40 KB binary claims
- AGENTS 5 tests vs STATUS 86 vs V1 102

---

## Tracking

- Update this file when a step gains in-tree tests (move to ✅ in ROADMAP Phase 11).
- Do not widen language surface without SPEC + tests.
- Prefer fixing compiler over loosening tests.
