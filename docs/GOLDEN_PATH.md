# Golden Path — post Niche-10 / Phases 21–24

**Status:** **Plan locked (docs)** — execute only with named gates + ADRs.  
**Baseline:** `master` through Phase **24** (`Map[str,i64]` + `examples/12_…`); local tag **`v0.1.0`** (no push unless explicitly asked).  
**Honesty parents:** [AGENTS.md](../AGENTS.md), [NICHE10.md](NICHE10.md), [VERDICT.md](VERDICT.md),
[LEAD_AHEAD.md](LEAD_AHEAD.md), ADRs [0007](adr/0007-deferred-ui-frameworks.md)–[0017](adr/0017-map-str-i64-mono.md).

**Rule (non-negotiable):** a wave is ✅ only when its **named in-tree gate** is green
and SPEC/docs match behavior. Prefer fixing the compiler over loosening tests.
Never mark ROADMAP ✅ without evidence. Never invent End-style domain theater.

---

## 0. North star (what “done enough” means)

1. **Niche-10 stays certified** — no Absolute-10 vs Go/nginx without a *new* ADR.
2. **Agent-first surface stays honest** — MCP path-first + LSP depth; no in-lang
   `feature`/`skill` keywords ([ADR-0009](adr/0009-agent-contracts-toolchain.md)).
3. **Collections grow by mono, not theater** — additive `Vec`/`Map` monomorphs;
   refuse parametric `Vec[T]` until an Absolute/parametric ADR.
4. **Product demos are callable `.ryx` + RT gates** — not brochure stubs.
5. **Release is explicit** — local tag ≠ remote push ≠ GitHub Release.

### Done-enough for a “v0.1 public” (optional track R)

Only after user says push: remote tag + Release binaries + CHANGELOG section cut
from Unreleased. Not required to keep shipping Phases 25+.

---

## 1. Current baseline (already shipped)

| Band | Content | Evidence |
|------|---------|----------|
| 0–15 | Core language → WASM Node path | ROADMAP |
| 16–20 | Niche-10 path | [NICHE10.md](NICHE10.md) |
| 21 | MCP path-first remainder, match variants, example 11, CHANGELOG | [PHASE21.md](PHASE21.md) |
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

Revisit of R1–R8 requires a **new ADR** + acceptance gates before any wave starts.

---

## 3. Golden sequence (Phases 25 → 30)

Execute **in order** unless a wave is marked ∥ (parallel). Skip only with an ADR
amendment recorded in this file’s changelog section at bottom.

### Phase 25 — Str-map complete + editor outline *(default next)*

| Wave | Theme | Gate (named) | ADR |
|------|--------|--------------|-----|
| 0 | Lock this GOLDEN_PATH + ROADMAP pointer | file contains “Phase 25” | — |
| A | `Map[str, str]` mono (`map_str_str_*`) | `map_str_str_roundtrip` | **0018** (new) |
| B | `documentSymbol` LSP | `document_symbol_lists_fn` | — |
| C | Example: HTTP + `Map[str,str]` headers-shaped demo | `example_map_str_str_product_checks` | — |
| D | Contract `phase25_golden.contract.toml` + skill/AGENTS touch | `verify_phase25_golden_contract` | — |

**Out:** payload enums, parametric maps, push/release.

Plan doc when started: `docs/PHASE25.md`.

---

### Phase 26 — Agent polish + honesty refresh *(∥-friendly after 25-A)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | `textDocument/formatting` → fmt; prepareRename/highlight thin | `lsp_formatting_applies_fmt` | — |
| B | MCP parity audit (`slice` tool if missing) + path-first docs | `mcp_slice_or_documented_absence` | — |
| C | `std::crypto` facade for HMAC/AES (soft remains real) | `std_crypto_hmac_aes_import_ok` | SPEC touch |
| D | Suite5 multi-lang refresh artifact post-P24 | `suite5_post_p24_artifact_links` | — |
| E | VERDICT / END_PEER_GAP peer date refresh (ff-only End) | `verdict_peer_date_current` | — |

**Out:** MCP HTTP/SSE (→ 28), Absolute-10.

---

### Phase 27 — Language depth (ADR-gated)

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| 0 | ADR for payload enums **or** reject with written deferral update | ADR-0019 status | **0019** |
| A | If Accepted: nullary-payload `Some(T)` construct + match bind | `enum_payload_match_roundtrip` | 0019 |
| B | Struct fields: `bool` (and/or nested struct i64/str only) | `struct_bool_field_roundtrip` | SPEC |
| C | Optional: multiline strings (one syntax) | `multiline_str_roundtrip` | SPEC + maybe ADR-0001 note |

**Hard stop:** if 0019 stays Deferred, skip A; do not stub `Some`.

---

### Phase 28 — Runtime / product depth *(no framework claim)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | uring recv/send: reduce poll fallback (Linux CI) | `uring_recv_send_completion_smoke` | — |
| B | OpenSSL-on CI path for TLS product (or documented stub matrix) | `tls_ci_matrix_documented` | — |
| C | Bounded HTTP auth-header or method extension (one feature) | `http_auth_or_method_gate` | SPEC soft table |
| D | Escape: one measured interprocedural improvement **or** doc limit | `escape_interproc_or_limit_doc` | — |

**Out:** nginx RPS, general TLS terminator, HTTP/2.

---

### Phase 29 — WASM / packages (honest ceiling)

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | Additional host-import(s) beyond `print_i64` (e.g. `print` str) | `emit_wasm_host_print_str` | — |
| B | Package UX polish (`new`/`deps` docs + one failing-path diag) | `package_ux_diag_gate` | — |
| C | Attest/local index honesty pass (still not Sigstore) | `attest_docs_match_impl` | 0010 |

**Out:** full WASI, CDN registry, Rekor — unless new ADR supersedes R5/R6.

---

### Phase 30 — Optional public v0.1 *(user-triggered only)*

| Wave | Theme | Gate | ADR |
|------|--------|------|-----|
| A | CHANGELOG cut: Unreleased → `[0.1.0]` full notes | file review | — |
| B | `git push` tag `v0.1.0` (explicit ask) | remote tag exists | — |
| C | GitHub Release via `release.yml` | release assets + SHA256SUMS | — |
| D | Optional GPG | documented | — |
| E | PRODUCTION_READINESS “public v0.1” row | honesty table | — |

**Default:** do **not** auto-start Phase 30.

---

## 4. Full backlog map (inventory → phase)

Every post-24 item from the continuation catalog, assigned.

### A — Ship/ops → Phase 30 (mostly)
A1 push tag · A2 GitHub Release · A3 GPG · A4 CHANGELOG cut · A5 peer refresh → **26-E** · A6 PR hygiene → **25-0 / 26** · A7 Marketplace → **after 30**, optional side-track.

### B — Language → Phase 27 (+ bits in 26)
B1 payload match → **27-A** · B2 `&T` → **post-30 / new ADR** (not default 27) · B3 struct fields → **27-B** · B4 multiline → **27-C** · B5 unicode idents → **post-30** · B6 crypto facade → **26-C** · B7 agent/signal/tensor implement → **Refuse** unless ADR · B8 eval CallExt → **post-30** (honesty-first).

### C — Collections → Phase 25 (+ later monos)
C1 `Map[str,str]` → **25-A** · C2 `Vec[bool]` → **post-25 additive** · C3 other monos → **on demand after 25** · C4 parametric → **Refuse** · C5 owned string keys → **after C1**, optional 25.1.

### D — LSP → Phase 25–26
D1 documentSymbol → **25-B** · D2 highlight/prepareRename → **26-A** · D3 signature/semantic → **post-26** · D4 formatting → **26-A** · D5 CodeLens depth → **low priority** · D6 Neovim recipes → **docs-only anytime**.

### E — MCP → Phase 26 (+ 28 later)
E1 path-first audit → **26-B** · E2 HTTP/SSE MCP → **post-28** · E3 verify contracts → **25-D / each phase** · E4 refuse End keywords → **standing** · E5 slice MCP → **26-B** · E6 scope UX → **26 docs**.

### F — HTTP/runtime → Phase 28
F1 deeper HTTP → **28-C** · F2 TLS terminator → **Refuse / post-30 ADR** · F3 uring poll → **28-A** · F4 OpenSSL CI → **28-B** · F5 HTTP/2 → **Refuse** · F6 WS app helpers → **post-28** · F7 interprocedural escape → **28-D**.

### G — WASM/packages → Phase 29
G1 host-imports → **29-A** · G2 WASI → **Refuse** · G3 CDN → **Refuse** · G4 Sigstore → **Refuse** · G5 sparse download → **when volume hurts** · G6 wasm UX → **29-B**.

### H — Benchmarks → Phase 26 + standing
H1 matrix vs End → **26-D notes** · H2 Suite5 refresh → **26-D** · H3 lexer SIMD → **post-30 research** · H4 SoA AST → **research** · H5 Suite12 divergent → **Refuse until oracle** · H6 SR disclosure → **standing** · H7 Absolute-10 → **Refuse**.

### I — Refuse → §2 table
All I1–I10 map to R1–R12.

---

## 5. Definition of Done (per phase)

1. `docs/PHASEnn.md` written before coding waves (except 25-0).
2. New language/RT surface: SPEC (+ ADR if irreversible).
3. Named `cargo test` gate(s) in `rynixc` (or crate) — green on Windows at least.
4. `docs/contracts/phasenn_*.contract.toml` + `verify_phasenn_*` test.
5. AGENTS.md / skill / CHANGELOG / ROADMAP updated in the same commit band.
6. No push/tag/release unless Phase 30 + explicit user ask.

---

## 6. Operating cadence

| Cadence | Action |
|---------|--------|
| Start phase | Write `PHASE*.md` → Wave 0 docs lock → implement A… |
| Each wave | One atomic commit preferred; gate must pass before ✅ |
| End phase | Contract verify green; ROADMAP table ✅ |
| Peer End | `fetch` / `pull --ff-only` only; never edit friend tree |
| Theater check | If a demo needs a stub domain row → stop; write ADR refuse |

---

## 7. Immediate next action (execute now)

**Start Phase 25** as written in §3:

1. Create `docs/PHASE25.md` + ADR-0018 draft for `Map[str,str]`.
2. Implement Wave A → B → C → D.
3. Commit locally; **do not push**.

When Phase 25 closes, open Phase 26 without re-litigating the refuse set.

---

## 8. Plan changelog

| Date | Change |
|------|--------|
| 2026-08-25 | Initial golden path locked from post-P24 full inventory (~45 items → Phases 25–30 + refuse). |
