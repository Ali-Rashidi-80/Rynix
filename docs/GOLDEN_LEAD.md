# Golden Lead — post Remaining SoT (Phases 39–49)

**Status:** **Lead Platform Complete** (Phases 39–46 + Wave 0).  
**Parent (closed):** [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md) (Phases 30–38, `v0.1.1`, `v0.2.0`).  
**Peer audit pin:** End @ **`bdc8732`** (2026-08-27) — read-only `D:\0\End-peer`.  
**Honesty parents:** [AGENTS.md](../AGENTS.md), [NICHE10.md](NICHE10.md), [VERDICT.md](VERDICT.md).

---

## North star

After Remaining 30–38, Rynix leads on **agent/IDE toolchain depth** (MCP-19, NDJSON diags,
contracts, scope gate, LSP) and **honest systems shipping** (real TLS/HTTP/WS, fibers,
freestanding WASM). End peer now ships real rustls TLS and h2 HTTP/2 in `endc` host Rust —
**do not sell “End TLS is fake.”** Sell **MCP + Suite5 checksum + honesty + Niche-10**.

Industry 2025–26: winning pattern = **Rust MCP sidecar + Python orchestration** / AI DSLs
that compile to existing stacks — not mass adoption of a new GP language. Rynix value =
**compiler/toolchain as MCP for `.ryx`**, not “everyone writes Rynix.”

---

## Lead Platform Complete (definition)

**Complete =** Wave 0 + Phases **39 + 40 + 41 + 42 + 43 + 46-pre + 46 checklist**.

| Included | Excluded (by design) |
|----------|----------------------|
| SoT + peer honesty refresh | seccomp / MSan (Track-R optional) |
| LSP prepareRename + documentHighlight | Horizon C `&T`/traits (ask-only) |
| ADR-0026 + `Option[T]` ship (i64\|str) | UI / C11 / Raft / End-DSL / full WASI |
| inlayHint from sema | Absolute-10 / Niche-11 / playground |
| `mcp/` split + Agent-Quality Pack (19 tools) | MCP≥20 empty tools |
| MCP dual-era / `server/discover` readiness | Streamable HTTP (Track-L ask) |

Gate: `golden_lead_sot` (Wave 0) then per-phase contracts below.

---

## Horizons

```text
Horizon A (Core):     Wave 0 → 39 → 40 → 41
Horizon B (Platform): 42 ∥ 43 → 46-pre → 46
Track-R (optional):   seccomp, MSan — after 46 or parallel; never blocks Complete
Track-L (ask-only):   Streamable HTTP, WASM host deepen, Suite5 matrix, std .ryx bodies
Horizon C (ask-only): 47 &T ADR → 48 ship; 49 traits ADR (parallel 47)
```

---

## Phase table

| Phase | Theme | Gate / contract |
|------:|-------|-----------------|
| **0** ✅ | SoT + peer `bdc8732` + doc drift | `golden_lead_sot` |
| **39** ✅ | LSP thin: prepareRename, documentHighlight | `phase39_lsp_thin` |
| **40** ✅ | ADR-0026 `Option[T]` allow-list i64\|str | `phase40_option_adr` |
| **41** ✅ | Ship `Option[T]` mono → payload RT | `phase41_option_t` |
| **42** ✅ | inlayHint (`def_types`/Local\|Param) | `phase42_inlay` |
| **43** ✅ | `mcp/` split + annotations + outputSchema | `phase43_mcp` |
| **46-pre** ✅ | MCP dual-era / discover | `mcp_dual_era` |
| **46** ✅ | Platform Complete checklist | `phase46_platform` |
| 44 | seccomp smoke (Linux skip) | optional |
| 45 | MSan deferral named or limited job | optional |
| 47–49 | `&T`, traits | explicit ask each |

---

## Gap closure matrix (Rynix-native — no End copy)

| Gap | Approach | Phase | End copy? |
|-----|----------|-------|-----------|
| Stale Gap/VERDICT (`cf5bef3`, TLS simulated) | Rewrite for `bdc8732`; compete on MCP+honesty | 0 | — |
| README MCP table 18 / missing `rynix_slice` | Full 19-tool table | 0 | — |
| Suite5 one-liner 10–2 vs table 11–1 | Align to **11–1** (`matrix`) | 0 | — |
| PHASE28/skill Track G drift | Historical labels + pointers here | 0 | — |
| LSP rename without prepare | prepareProvider + highlight | 39 | — |
| No parametric `Option[T]` | mono i64\|str on ADR-0024 RT | 40–41 | — |
| No inlayHint | sema types at bindings | 42 | — |
| Monolithic MCP + old protocol surface | split + quality pack + dual-era | 43, 46-pre | — |
| End HTTP/2 / PG / SQLite / GPU | Fate: FFI-later or Refuse | 0 README | **Refuse core** |
| End agent DSL keywords | MCP + contracts (ADR-0009) | — | **Refuse** |
| Remote MCP HTTP | localhost Streamable HTTP | Track-L | — |

---

## Domain Fate (vs End @ bdc8732)

Evidence-gated rows — **do not copy End 🟢 Stable brochure.**

| Domain | End @ bdc8732 | Rynix | Fate |
|--------|---------------|-------|------|
| MCP | **Absent** | 19 tools stdio | **Lead** |
| LSP + IDE | Partial; no LanguageClient | LanguageClient + depth path | **Lead** |
| TLS (product) | rustls in host; C path partial | SChannel/OpenSSL product | **Ship** (parity; not “we win TLS”) |
| HTTP/2 | h2 in host Rust | HTTP/1 + WS product | **FFI-later / low priority** |
| Suite5 checksum CI | Different harness | 11–1 vs End on same kernels | **Lead** |
| Postgres / Redis / SQLite | tokio-postgres / toy redis / rusqlite | — | **FFI-later / Refuse rewrite** |
| Raft / GPU / GGUF | runtime raft / wgpu / candle | ADR deferred / stubs rejected | **Refuse** |
| UI / C11 / CDN | marketed Stable | ADR deferred | **Refuse** |
| Agent DSL `feature/skill` | language keywords | toolchain contracts | **Refuse keywords** |

---

## Permanent Refuse (growth path)

stubs RYX2013 · UI ADR-0007 · C11 ADR-0008 · Raft ADR-0012 · End DSL ADR-0009 ·
full WASI/mobile · Absolute-10 / Niche-11 / playground · Sigstore theater ·
MCP≥20 empty tools · OAuth IdP inside rynixc · copying End domain wallpaper.

Reopen = amend this file + ADR + explicit ask.

---

## Track-L backlog (post-Complete, ask-only)

| Item | Industry need |
|------|----------------|
| Streamable HTTP MCP (localhost, Origin-checked) | remote/team agents |
| WASM `env.*` deepen | agent sandboxes without WASI claim |
| Suite5 `matrix` kernel strategy | single peer loss row |
| Real `std/*.ryx` bodies over CallExt | agent-readable source |
| fiber stats as MCP Resource | runtime transparency without tool #20 |

---

## Anti micro-plan rule

1. All Lead work lives in **this file** after Wave 0.
2. `PHASE*.md` per phase allowed; no new `golden_*_path` unless Absolute ask.
3. ROADMAP ✅ only with in-tree gate green.

---

## Execution order (after plan approval)

1. Wave 0 (honesty first)
2. 39 → 40 → 41
3. 42 ∥ 43
4. 46-pre → 46 → natural stop
5. Track-R / Track-L / 47–49 only on ask
