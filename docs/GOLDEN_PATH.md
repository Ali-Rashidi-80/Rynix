# Golden Path — Quality 10 (post Niche-10 / Phases 21–24)

**Status:** **Plan locked (docs)** — execute only with named gates + ADRs.  
**Sources absorbed (2026-08-26):** Desktop
`Rynix_Analysis_Report.html` (score **8.7/10**) +
`Rynix_Golden_Quality_Plan.html` (90-day A–E / 56 tasks) — **filtered through
honesty**, not copied wholesale.  
**Baseline:** `master` through Phase **24** (`Map[str,i64]` + `examples/12_…`);
local tag **`v0.1.0`** (no push unless explicitly asked).  
**Honesty parents:** [AGENTS.md](../AGENTS.md), [NICHE10.md](NICHE10.md),
[VERDICT.md](VERDICT.md), [LEAD_AHEAD.md](LEAD_AHEAD.md),
ADRs [0007](adr/0007-deferred-ui-frameworks.md)–[0017](adr/0017-map-str-i64-mono.md).

**Rule (non-negotiable):** a wave is ✅ only when its **named in-tree gate** is
green and SPEC/docs match behavior. Prefer fixing the compiler over loosening
tests. Never mark ROADMAP ✅ without evidence. Never invent End-style domain
theater.

---

## 0. North star — what “10/10 quality & maturity” means

**Quality-10** here is **engineering maturity**, not Absolute-10 vs Go/nginx
and not “parametric generics shipped or fail.”

| Must hold | Meaning |
|-----------|---------|
| Niche-10 stays certified | No Absolute-10 marketing without a *new* ADR |
| Agent surface stays honest | MCP path-first + LSP depth; no in-lang `feature`/`skill` |
| Collections grow by mono **until** a parametric ADR is **Accepted** | `Map[str,str]` etc. additive; refuse `Vec[T]` theater |
| Product demos are callable `.ryx` + RT gates | Not brochure stubs |
| Release is explicit | Local tag ≠ remote push ≠ GitHub Release |
| Max maintainability | Hot files split (target ≤ ~900 LOC/file after decompose) |
| Security posture | Build subprocess risk documented + optional sandbox + IR sanitize gates |
| Zero TODO/FIXME | Preserve; deferred work → ADR only |

### Scoreboard (analysis report → Quality-10 targets)

| Axis | Report | Target ≥ | Primary waves |
|------|--------|----------|---------------|
| Architecture | 9.4 | 9.5 | keep + ADR discipline |
| Rust code quality | 8.6 | **9.5** | **26** lower/lsp split, unwrap budget |
| C runtime quality | 8.2 | **9.0** | **27** MSan/UBSan + fuzz |
| Test strategy | 9.2 | 9.5 | **27** fuzz expand |
| Error handling | 8.8 | 9.2 | **26** unwrap audit |
| **Security** | **7.6** | **≥9.0** | **27** sandbox + sanitize + threat model |
| Performance | 9.0 | 9.2 | **29** optional; not a Quality-10 blocker |
| Deployment / CI | 8.6 | 9.5 | **26** sanitize scaffold; **30** release |
| AI tooling | 9.4 | 9.6 | **25** documentSymbol; **28** format/MCP |
| Documentation | 9.4 | 9.6 | **28–29** user-facing Book skeleton |
| Niche-10 axe | 9.0 | stay certified | no Niche-11 theater without gates |

**Weighted Quality-10:** every axis ≥ **9.0**, Security ≥ **9.0**, unwrap
budget gate green, decompose gates green, Niche-10 still certified.
Performance may stay at report **9.0** without Q-Full D-* work.
Deployment ≥9.0 under Q-Core = CI sanitize scaffold (**26-F**) + honesty docs
(**27-E**, PRODUCTION_READINESS); remote Release (**30**) raises it further but
is **not** required for Q-Core lock.
**Does not require:** 1 GiB/s lexer, GitHub stars, Coq fiber proof,
parametric `Vec[T]`, or tarpaulin ≥80% (R18).

**Axis re-score method (non-theater):** when a phase claims an axis move, update
a one-row table in that phase’s `PHASE*.md` with: prior score, new score,
**named gate(s)** that justify the delta. No gate → no score change.

### Capacity honesty (from Quality Plan appendix)

The HTML 90-day plan estimates **~744h**; ~20h/week × 13 ≈ **227h**. Gap is real.

| Path | Hours/wk | Outcome | Default? |
|------|----------|---------|----------|
| **Q-Core (recommended)** | ~20 | Axes ≥9.0; Phases 25–29; Phase 30 user-triggered; parametric/generics **ADR track only** | **Yes** |
| Q-Full | ~30 | Q-Core + perf bakeoffs (old Phase D) + optional formal-verify research | Opt-in |
| Absolute track | after ADR | Parametric generics / traits / Niche-11 — **never default waves** | No |

---

## 1. Current baseline (already shipped)

| Band | Content | Evidence |
|------|---------|----------|
| 0–15 | Core language → WASM Node path | ROADMAP |
| 16–20 | Niche-10 path | [NICHE10.md](NICHE10.md) |
| 21 | MCP path-first remainder, match variants, example 11 | [PHASE21.md](PHASE21.md) |
| 22 | Inline match+return CFG fix; MCP format/compile path-first | [PHASE22.md](PHASE22.md) |
| 23 | LSP refs/symbols; `Enum::Variant`; `Vec[str]`; local `v0.1.0` | [PHASE23.md](PHASE23.md) |
| 24 | `Map[str,i64]`; example 12 | [PHASE24.md](PHASE24.md) |

---

## 2. Refuse set (never schedule as default waves)

| ID | Refuse | ADR / SoT |
|----|--------|-----------|
| R1 | UI / canvas / hot-reload / wgpu studio | [0007](adr/0007-deferred-ui-frameworks.md) |
| R2 | C11 stub transpile backend | [0008](adr/0008-deferred-c11-backend.md) |
| R3 | Raft / consensus “Stable” product | [0012](adr/0012-deferred-consensus.md) |
| R4 | llama / GGUF reimplemented in `.ryx` | [NICHE10.md](NICHE10.md) |
| R5 | CDN-required registry / Sigstore theater | [0010](adr/0010-local-package-index.md) |
| R6 | Full WASI / `rt/` in wasm as “done” | PHASE14–15 / NICHE10 |
| R7 | Absolute-10 vs Go/nginx marketing | [0013](adr/0013-niche-10-scorecard.md) |
| R8 | HTTP/2 / Cranelift / PubGrub stubs | END_PEER_GAP / LEAD_AHEAD |
| R9 | End `feature`/`skill` language keywords | [0009](adr/0009-agent-contracts-toolchain.md) |
| R10 | ROADMAP ✅ / SPEC widen without gates | AGENTS honesty |
| R11 | Suite12 #1/#5/#6 without C oracle checksum | [0011](adr/0011-suite12-divergent-benches.md) |
| R12 | Claim identical asm after Suite5 strength reduction | AGENTS / Suite5 Notes |
| R13 | Parametric `Vec[T]` / `Map[K,V]` / traits as **default** waves | Needs **new Accepted ADR**; not ADR-0018 |
| R14 | Niche-11 / “stars ≥1K” / playground CDN as Quality-10 gates | Marketing ≠ maturity |
| R15 | Coq/Isabelle fiber proof as **required** Quality-10 | Optional research; B-5 HTML → Track R |
| R16 | Steal ADR-0018 for “Generics design” | **0018 = `Map[str,str]` mono only** |
| R17 | Mobile NDK/iOS / Discord-as-gate / chat-platform marketing | LEAD_AHEAD §0b; Track C is docs/process only |
| R18 | HTML **KPI-1** tarpaulin ≥80% as Quality-10 gate | Prefer targeted gates; coverage % → Track R optional |
| R19 | llama.cpp **FFI product** as default wave | R4 covers reimpl; honest FFI+smoke → Track R only |

Revisit of R1–R8 / R13 / R17–R19 requires a **new ADR** + acceptance gates before any wave starts.

---

## 3. Analysis → Golden map (10 weaknesses)

From `Rynix_Analysis_Report.html` §29–30:

| # | Weakness | Golden disposition |
|---|----------|--------------------|
| 1 | Monomorphic collections | **25-A** `Map[str,str]`; more monos on demand; parametric → **Track G** (ADR) |
| 2 | Textual LLVM only (ADR-0005) | Keep; mitigate with **27** sandbox + sanitize — do **not** replace with in-process LLVM |
| 3 | No build subprocess sandbox | **27-A/B** `--sandbox=docker\|none` + IR sanitize |
| 4 | `lower.rs` ~5k LOC | **26-A** split `lower/` (behavior-identical) |
| 5 | `lsp_cmd.rs` ~1.6k LOC | **26-B** split `lsp/`; lsp-types migrate = **Track R** (dep ADR) |
| 6 | unwrap/expect in src | **26-C** audit + budget gate |
| 7 | Minimal std `.ryx` | Soft stays; real defs after generics ADR or thin facades in **28-C** |
| 8 | C runtime / asm | **27-C** MSan/UBSan + fuzz; formal proof → Track R |
| 9 | `repository = example.invalid` | **Done** — `Cargo.toml` → `github.com/Ali-Rashidi-80/Rynix` |
| 10 | Single-dev cadence | **26-E** one-phase contract discipline; no 24-phases-in-4-days |

HTML Quality Plan tasks **A-1…E-15** map into Phases **25–30** + Tracks below — **not** a second competing roadmap.

---

## 4. Golden sequence (Phases 25 → 30)

Execute **in order** unless marked ∥. Skip only with an ADR amendment in §10.

### Phase 25 — Str-map + editor outline + Quality lock *(default next)*

Closes analysis weakness #1 (next mono) and AI-tooling depth; locks this plan.

| Wave | Theme | Gate (named) | ADR |
|------|--------|--------------|-----|
| 0 | Lock this GOLDEN_PATH + ROADMAP pointer | file contains “Quality-10” | — |
| A | `Map[str, str]` mono (`map_str_str_*`) | `map_str_str_roundtrip` | **0018** |
| B | `documentSymbol` LSP (+ VS Code client capability if needed) | `document_symbol_lists_fn` | — |
| C | Example: HTTP + `Map[str,str]` headers-shaped demo | `example_map_str_str_product_checks` | — |
| D | Contract `phase25_golden.contract.toml` + skill/AGENTS touch | `verify_phase25_golden_contract` | — |

**Out:** payload enums, parametric maps, push/release, ADR-0018-as-generics.

Plan doc when started: `docs/PHASE25.md`.

---

### Phase 26 — Maturity decompose *(HTML A-3…A-8; weakness 4–6, 9–10)*

**No language surface widen** except docs/process.

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | Split `rynix-rir` `lower.rs` → `lower/` (≤~900 LOC/file) | `lower_decomp_invariants` (existing rir tests + snapshots identical) | **0019** (decomp) |
| B | Split `lsp_cmd.rs` → `lsp/` | `lsp_decomp_parity` (JSON-RPC corpus identical) | **0020** |
| C | unwrap/expect in `crates/*/src` (non-test) **N ≤ 60** (HTML A-5) | `unwrap_budget_gate` | — |
| D | Repository URL when known (`repo_url_real`); CODEOWNERS/issue templates → Track C / **29-I** | `repo_url_real` *or* documented placeholder deferral | — |
| E | Phase-contract schema + CI template (one phase / PR discipline) | `contract_schema_gate` | **0021** |
| F | Sanitizer CI scaffold (continue-on-error ok) + clippy discipline | `sanitizer_scaffold_documented` | — |
| G ∥ | `cargo deny` advisories *or* documented deferral (HTML B-8; not sanitizer) | `cargo_deny_or_deferral` | — |

**Out:** behavior changes, lsp-types dep (→ Track R), remote push.
Unsafe-block count: red-flag only — **do not increase** beyond current src count; no separate wave.

---

### Phase 27 — Security posture *(HTML A-2, B-1…B-4, B-7; weakness 2–3, 8)*

Raises Security axis from **7.6 → ≥9.0**.

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | `--sandbox=docker\|none` for clang link (opt-in; default `none` until docs) | `sandbox_docker_smoke` *or* documented skip matrix | **0022** |
| B | RIR/LLVM sanitize: reject `system`/`exec*`/`popen`/`dlopen` escapes | `sanitize_rejects_exec` | **0023** |
| C | MSan+UBSan enforce on `rt/` smokes (Linux CI) | `msan_ubsan_rt_clean` | — |
| D | Fuzz targets: parse + sema + rir-interp (seed corpus) | `fuzz_new_targets_seeded` | — |
| E | Threat model doc (STRIDE) under `docs/SECURITY_THREAT_MODEL.md` | file + link from SECURITY.md | — |
| F ∥ | Windows Job Object sandbox *or* documented deferral | `windows_sandbox_or_deferral` | 0022 amend |
| G ∥ | `emit-ll` / no-clang-link fast path smoke (analysis #2 DX) | `emit_ll_no_link_smoke` | — |
| H ∥ | CWE matrix beyond 798: document current scanner scope + one additive class *or* deferral (HTML B-10) | `security_cwe_matrix_or_deferral` | — |

**Out:** seccomp as hard Quality-10 requirement (nice-to-have after A); Coq fiber proof; claim “untrusted .ryx is safe.”

---

### Phase 28 — Agent polish + language depth (ADR-gated) *(HTML E-8/E-9 + old 26/27)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | `textDocument/formatting` → fmt; thin highlight/prepareRename; wire VS Code client if capability shipped | `lsp_formatting_applies_fmt` | — |
| B | MCP parity audit (`slice` tool if missing) + path-first docs | `mcp_slice_or_documented_absence` | — |
| C | `std::crypto` facade HMAC/AES (soft remains real) | `std_crypto_hmac_aes_import_ok` | SPEC |
| D | ADR for payload enums **or** written deferral | ADR-0024 status | **0024** |
| E | If Accepted: nullary-payload `Some(T)` + match bind | `enum_payload_match_roundtrip` | 0024 |
| F | Struct `bool` field (and/or nested i64/str only) | `struct_bool_field_roundtrip` | SPEC |
| G | VERDICT / END_PEER_GAP peer date refresh (ff-only End) | `verdict_peer_date_current` | — |
| H ∥ | Optional: multiline strings (one syntax) | `multiline_str_roundtrip` | SPEC |

**Hard stop:** if 0024 stays Deferred, skip E; do not stub `Some`.

**Out:** MCP HTTP/SSE, Absolute-10, parametric collections; codeAction/inlayHints (post-28); MCP≥20 theater without real tools.
---

### Phase 29 — Runtime / WASM / docs ceiling *(HTML D-lite + E-1 skeleton + old 28/29)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | uring recv/send: reduce poll fallback (Linux CI) | `uring_recv_send_completion_smoke` | — |
| B | OpenSSL-on CI path for TLS **or** stub matrix doc | `tls_ci_matrix_documented` | — |
| C | Bounded HTTP auth-header or method (one feature) | `http_auth_or_method_gate` | SPEC soft |
| D | Escape: one measured interprocedural win **or** limit doc | `escape_interproc_or_limit_doc` | — |
| E | WASM host-import beyond `print_i64` (e.g. print str) | `emit_wasm_host_print_str` | — |
| F | Package UX + attest honesty pass (not Sigstore) | `package_ux_diag_gate` + `attest_docs_match_impl` | 0010 |
| G | Rynix Book **skeleton** (3+ chapters, links to SPEC/examples) + tutorial outline (E-6) | `book_skeleton_exists` | — |
| H ∥ | Suite5 refresh artifact post-collections | `suite5_post_p24_artifact_links` | — |
| I ∥ | Track C kickoff: RFC template **or** CONTRIBUTING sections (E-3/E-4) | `rfc_or_contributing_sections` | — |

**Out:** nginx RPS bakeoff as Quality gate, 1 GiB/s lexer requirement, full WASI, CDN.
---

### Phase 30 — Optional public v0.1 *(user-triggered only; HTML A-1)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | CHANGELOG cut: Unreleased → `[0.1.0]` | file review | — |
| B | `git push` tag `v0.1.0` (explicit ask) | remote tag exists | — |
| C | GitHub Release via `release.yml` | release assets + SHA256SUMS | — |
| D | Optional GPG | documented | — |
| E | PRODUCTION_READINESS “public v0.1” + Quality-10 scoreboard row | honesty table | — |

**Default:** do **not** auto-start Phase 30.

---

## 5. Side tracks (not default phases)

### Track G — Parametric generics *(HTML Phase C; R13)*

Only after a **dedicated Accepted ADR** (number **after** 0024; **not** 0018):

- Design: monomorphization, TypeKind, parser, RT, retire legacy monos
- Gates: `vec_t_roundtrip`, `map_kv_roundtrip`, …  
- Until then: keep shipping **monos** (`Vec[bool]`, owned string keys, …) under Phase 25+ additive waves.

### Track R — Research / optional hardening *(HTML B-5, B-6, D-1…D-11, E-10)*

| HTML | Item | Note |
|------|------|------|
| B-1 | seccomp-bpf on clang | After docker sandbox; Linux-first |
| B-5 | Formal fiber_swap proof | Optional; not Quality-10 blocker |
| B-6 | lsp-types / tower-lsp | New dep → ADR-0004 justification |
| D-1 | PGO pipeline | Q-Full only |
| D-2 | Inline hints | Q-Full |
| D-3 | Region v2 | Q-Full |
| D-4 | SIMD 1 GiB/s lexer | Q-Full; **not** Quality-10 gate |
| D-5 | Compile-time bench | Q-Full (analysis strategic #7) |
| D-6 | Binary size budget | Q-Full |
| D-7 | HTTP RPS bakeoff | Refuse Absolute marketing; research only |
| D-8 | AST pool / bumpalo tuning | Q-Full |
| D-9 | LLVM stream / emit daemon | Optional DX; see also **27-G** `--emit-ll` path |
| D-10 | LSP incremental | post-28 additive |
| D-11 | Perf regression CI | Q-Full |
| E-2 | Playground | Marketing; not maturity gate |
| E-10 | External security audit | After **27**; optional paid review |
| B-10 | CWE expand beyond 798 | **27-H** (matrix or deferral) |
| E-7 | API docgen tool | After Track G / real std defs |
| — | `mcp_cmd.rs` split | Mirror **26-B** pattern; additive anytime |
| — | llama.cpp FFI (honest) | Track R; default refuse **R19** |
| — | Honest Sigstore @ scale | Track R / post-adoption; theater stays **R5** |
| — | HTML KPI-1 tarpaulin ≥80% | Track R optional; Quality refuse **R18** |
| — | Mobile NDK/iOS | Refuse **R17** |

### Track C — Community / process *(HTML E-3…E-6, E-14…E-15; weakness #10)*

Not language surface. Schedule **after Phase 26-E** or ∥ with **29**; never blocks Quality-10 axes.

| HTML | Item | Disposition |
|------|------|-------------|
| E-3 | RFC process (`rfcs/` + template) | Track C; before Track G language widen |
| E-4 | CONTRIBUTING expand + good-first-issue labels | Track C; ties to **26-D** templates |
| E-5 | ADR index / backfill hygiene | Standing — each phase; ≥25 ADRs is aspirational not gate |
| E-6 | 5-part tutorial (runnable examples) | **29-G** companion under Book skeleton |
| E-14 | Onboard external contributors | Process only; **not** a cargo-test gate |
| E-15 | 90-day retrospective + v0.3 notes | After Q-Core close or Phase **30-E** |

### Track v0.2 — only after Quality-10 scoreboard green + user ask

Do **not** schedule “v0.2.0 with generics” as Phase 30. v0.2 = Track G Accepted + gates + explicit release ask.

---

## 6. Full backlog map (inventory → phase)

### HTML A–E complete checklist (56/56)

| ID | Disposition |
|----|-------------|
| A-1 | **30** (explicit ask) |
| A-2 | **27-A** |
| A-3 | **26-A** |
| A-4 | **26-B** |
| A-5 | **26-C** |
| A-6 | **26-D** |
| A-7 | **26-E** |
| A-8 | **26-F** → enforce **27-C** |
| B-1 | Track R |
| B-2 | **27-B** |
| B-3 | **27-C** |
| B-4 | **27-D** |
| B-5 | Track R |
| B-6 | Track R |
| B-7 | **27-E** |
| B-8 | **26-G** `cargo_deny_or_deferral` (not sanitizer 26-F) |
| B-9 | **27-F** |
| B-10 | **27-H** `security_cwe_matrix_or_deferral` |
| C-1 | **Track G** ADR design (not 0018) |
| C-2 | **Track G** TypeKind refactor |
| C-3 | **Track G** parser generics |
| C-4 | **Track G** monomorphization |
| C-5 | **Track G** runtime generic |
| C-6 | **25-A** (mono; ADR-0018) — **exception** |
| C-7 | **Track G** HM-lite inference |
| C-8 | **Track G** full patterns; partial payloads via **28-D/E** |
| C-9 | **Track G** trait system v0 |
| C-10 | **Track G** retire legacy monos |
| C-11 | **Track G** std real `.ryx` defs (thin facades may land in **28-C**) |
| C-12 | **Track G** diff / compatibility tests |
| D-1…D-11 | Track R / Q-Full (table §5) |
| E-1 | **29-G** Book skeleton |
| E-2 | Track R |
| E-3…E-6, E-14…E-15 | **Track C** |
| E-7 | Track R (after std richness) |
| E-8 MCP 20+ | **28-B** parity first; generics tools → Track G |
| E-9 LSP 10+ | **25-B** documentSymbol; **28-A** format; codeAction/inlay → post-28 |
| E-10 | Track R |
| E-11 Niche-11 | **R14** refuse until ADR |
| E-12 | End **29** self-scoreboard + **30-E** PRODUCTION_READINESS Quality-10 row (external audit = E-10 Track R) |
| E-13 v0.2.0 | Track v0.2 + user ask |

### Analysis report — 8 strategic suggestions

| # | Suggestion | Disposition |
|---|------------|-------------|
| 1 | Stabilize v0.1 before v0.2 | **30** + Phase **26** cadence |
| 2 | Sandbox build subprocess | **27-A/B** |
| 3 | Refactor giant files | **26-A/B** (+ mcp split Track R) |
| 4 | Generics Phase 25+ | Mono **25**; parametric **Track G** |
| 5 | Expand fuzzing | **27-D** |
| 6 | User-facing docs | **29-G** + Track C E-6 |
| 7 | Perf profiling / compile bench | Track R D-5; not Quality-10 blocker |
| 8 | Community building | **Track C** |

### Analysis weakness #2 extra: `--emit-ll` / no-link fast path

| Wave | Add to Phase 27 | Gate |
|------|-----------------|------|
| G ∥ | Document + gate existing `emit-ll` / build `--emit-only` style path (no new LLVM backend) | `emit_ll_no_link_smoke` |

Do **not** invent an in-memory clang daemon as a Quality-10 requirement (D-9 stays Track R).

### Prior language / runtime map (unchanged intent)

- Payload match → **28-D/E** · struct fields → **28-F** · multiline strings → **28-H** (optional SPEC)  
- `&T` / unicode idents / eval CallExt → post-30 ADR  
- uring / TLS CI / HTTP soft → **29-A…C** · WASM host-import → **29-E** · packages/attest → **29-F**  
- Suite5 refresh → **29-H** · peer VERDICT → **28-G** · `Vec[bool]` mono → post-25 additive
---

## 7. Definition of Done (per phase)

1. `docs/PHASEnn.md` written before coding waves (except 25-0).
2. New language/RT/security surface: SPEC and/or ADR if irreversible.
3. Named `cargo test` (or script) gate(s) — green on Windows at least (Linux where noted).
4. `docs/contracts/phasenn_*.contract.toml` + `verify_phasenn_*` test when phase adds surface.
5. AGENTS.md / skill / CHANGELOG / ROADMAP updated in the same commit band.
6. No push/tag/release unless Phase 30 + explicit user ask.
7. Quality-10 scoreboard updated in PHASE doc when a phase claims an axis move.

---

## 8. Operating cadence

| Cadence | Action |
|---------|--------|
| Start phase | Write `PHASE*.md` → Wave 0 docs lock → implement A… |
| Each wave | One atomic commit preferred; gate must pass before ✅ |
| End phase | Contract verify green; ROADMAP table ✅ |
| Weekly | `cargo test --workspace`; unwrap budget script if present; no new TODO/FIXME |
| Peer End | `fetch` / `pull --ff-only` only; never edit friend tree |
| Theater check | If a demo needs a stub domain row → stop; write ADR refuse |
| Red flags | TODO/FIXME appears; unsafe src blocks > budget; Suite5 checksum fail; milestone slip >2 weeks |

---

## 9. Immediate next action (execute now)

**Phase 25 closed.** Next: **Phase 26** (maturity decompose) per §4.

Do **not** start Track G / Phase 30 without explicit user direction + ADR.

---

## 10. Plan changelog

| Date | Change |
|------|--------|
| 2026-08-25 | Initial golden path locked from post-P24 inventory (~45 items → Phases 25–30 + refuse). |
| 2026-08-26 | **Quality-10 absorb:** Analysis Report (8.7, 10 weaknesses) + Golden Quality Plan (A–E) merged; Security/decompose/fuzz elevated; parametric generics → Track G; ADR-0018 reserved for `Map[str,str]`; capacity Q-Core default; refuse R13–R16. |
| 2026-08-26 | **Subagent audit close:** R17–R19; unwrap N=60; B-8→26-G; B-10→27-H; axis re-score method; Deployment Q-Core without Phase 30; VS Code wire note on 25-B/28-A; KPI-1 refuse; LEAD_AHEAD re-entry mapped. |

---

## 11. Final coverage audit (do-not-ship-without)

**Verdict (post subagent round):** inventory **PASS** (56/56 + 10 weaknesses + 8 strategic).
Gate specificity gaps from audit are **closed** in §0 / §2 / §4 / §6.
Q-Core plan is **complete enough to execute Phase 25**.

| Source | Count | Status |
|--------|------:|--------|
| Analysis 10 weaknesses | 10 | All in §3 |
| Analysis 8 strategic | 8 | All in §6 |
| HTML tasks A-1…E-15 | 56 | All in §6 checklist |
| Scoreboard 11 axes | 11 | All in §0 + re-score method |
| Capacity honesty | 2 paths | §0 Q-Core default |
| Subagent todos | plan-hygiene + Phase 25–30 | See Cursor plan |

**Ready to execute Phase 25** when user says start.
