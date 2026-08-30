# Verdict — Rynix vs End (who is ahead?)

**Date:** 2026-08-30  
**Peer:** [IrMaho/End](https://github.com/IrMaho/End) `main` @ **`bdc8732`**  
(local clone `D:\0\End-peer`, `git fetch` + `pull --ff-only` — **no peer source edits**)  
**This repo:** Rynix Golden Remaining closed (30–38); **Lead path:** [GOLDEN_LEAD.md](GOLDEN_LEAD.md);  
Niche-10 certified ([NICHE10.md](NICHE10.md)).

This document answers one question under audit rules:

> Is continuing Rynix a waste of time relative to End, or is Rynix already
> ahead on what actually ships?

---

## One-line answer

**Rynix is ahead on the shipping agent/systems core End cannot match today:**
MCP-19, contracts/scope, NDJSON diags, VS Code LanguageClient depth path, Suite5
checksum CI, honest deferred ADRs. End @ `bdc8732` now ships **real rustls TLS and
h2 HTTP/2** in host Rust — **stop claiming “End TLS is simulated.”** End still leads
**spectacle** (README domain wallpaper, ~60 CLI names, agent DSL keywords, C11/UI
narrative) — surfaces we refuse to fake ([ADR-0007](adr/0007-deferred-ui-frameworks.md),
[0008](adr/0008-deferred-c11-backend.md), [0009](adr/0009-agent-contracts-toolchain.md)).

You are **not** behind on real language value. Time spent on Rynix built
auditable depth; End’s green tables often describe stubs.

---

## Judgment rules (how we score)

| Rule | Meaning |
|------|---------|
| Code > README | A 🟢 row without tests/RT is theater |
| Same harness | Suite5 ms only vs Suite5; never Suite5 vs End suite12 |
| Strength reduction disclosed | Faster checksum-same binary ≠ identical asm |
| Deferred ≠ loss | Refusing fake UI/C11/CDN is honesty, not defeat |
| Peer untouched | End clone is read-only for fair audit |

Full methodology: [END_PEER_GAP.md](END_PEER_GAP.md). Positioning: [COMPARE.md](COMPARE.md).

---

## Scorecard — shipping axes

| Axis | End @ bdc8732 | Rynix | Winner |
|------|---------------|-------|--------|
| Native binaries | **Real** C11 → host CC; host Rust runtimes | **Real** LLVM IR → clang ThinLTO | **Tie** product paths; Rynix size gates |
| Async / I/O runtime | Fibers + host tokio net | Fibers + **portable / uring / IOCP** | **Rynix** |
| TLS (product path) | **rustls** in host; C/std partial | Real SChannel / OpenSSL product | **Parity** — not “Rynix wins fake TLS” |
| HTTP/2 | **h2** in host Rust | HTTP/1 + WS product | **End host** on H2; Rynix refuses brochure chase |
| Crypto | SHA-256 + HMAC real; some AES/Argon theater | SHA + HMAC + AES-GCM KATs | **Rynix** (KAT discipline) |
| HTTP | Host HTTP/2 + thin `.end` | One-shot + **bounded loop** + JSON | **Rynix** product ergonomics |
| WebSocket | — / incomplete | RFC 6455 + wire smokes | **Rynix** |
| Agent CLI | ~60 names, uneven depth | graph/slice/impact/eval/patch/verify… | Tie overlap; End broader **names** |
| **MCP** | **Absent** | **19 tools** (`mcp-serve`, includes `rynix_slice`) | **Rynix** |
| Packages | Local path; **PubGrub theater** | Path + local index + unity/semver/lock/workspace + **local digest attest** | **Rynix** |
| WASM | Text / toy surface | `emit-ll --target=wasm32` + **`emit-wasm`** + Node run — **no WASI** | **Rynix** (honest subset) |
| Memory / escape | Real region/borrow subset | Escape + move + `#^ effect: pure` + explain-alloc | **Rynix** (transparency) |
| Editor LSP | LSP server; VS Code **without** LanguageClient | `lsp-serve` + **LanguageClient** + CodeLens + workspace goto | **Rynix** |
| Microbench fairness | suite12 (different programs) | Suite5 **C↔Rynix CI** + optional End slot | **Rynix** |
| Docs honesty | 62 files; STATUS often green on stubs | Evidence-gated maturity + ADRs | **Rynix** |
| Spectacle / domain matrix | Strong marketing | Refuses wallpaper | **End** (spectacle only) |

**Tally (shipping):** Rynix wins the axes that decide “usable systems + agent language.”  
**Spectacle:** End wins the brochure. Brochures do not compile TLS.

---

## Performance (Suite5 — same algorithms)

Harness: `benchmarks/suite5/` — identical integer kernels, opaque trip counts,
checksum parity required. End’s suite12 is a **different** program set — do not
cross-compare ms.

### vs C / Rust / Go / Zig (2026-08-25)

Rynix fastest on **11/12** (Zig edged `nested`). Rynix/C ratios ~0.19–0.98.
Large wins use **disclosed** strength reduction (same checksum). See
[END_PEER_GAP.md](END_PEER_GAP.md) §2 and
[benchmarks/suite5/README.md](../benchmarks/suite5/README.md).

### vs End on Suite5 (2026-08-25 Phase 16-A, this machine)

Peer `endc` built release-only from untouched `D:\0\End-peer` @ `bdc8732`
(`ENDC_PATH=…/endc/target/release/endc.exe`).

**Peer compiler regressions discovered during prior audit (not Rynix edits):**

1. Statement `if cond { … }` **fails to parse** — End’s own
   `benchmarks/suite12/suite12_end.end` errors with `Expected token Else, found LBrace`.
2. Expression `a if cond else b` **always evaluates to 0** in probed binaries.
3. Rynix-owned Suite5 `.end` ports use only `for` / `while` / `getenv as i64`
   (see `benchmarks/suite5/regen_end_ports.py` + `END_INTEGRATION.md`).

Live head-to-head: `benchmarks/suite5/suite5_summary_2026-08-25_phase16.txt`
(**Rynix 11 · End 1** on `matrix`) and table in [END_PEER_GAP.md](END_PEER_GAP.md) §2.
Checksums must match C on every counted End row.

Binary size (hello full RT gate): **&lt;300 KiB** (~13 KiB measured 2026-08-25).
Suite5 `--bench` Rynix bins were smaller than End `--strip` on prior Windows run.

---

## Doubt checklist (anxiety → evidence)

| Worry | Evidence response |
|-------|-------------------|
| “End README looks finished; maybe I’m late” | Host Rust @ bdc8732 is real on TLS/H2/DB; brochure rows still overclaim C/UI/registry |
| “End has more CLI commands” | Volume ≠ depth; Rynix has **MCP** (End has zero) |
| “End has C11 / UI / frameworks” | Real C transpile + host runtimes exist; copying wallpaper wastes months. We deferred C11/UI **on purpose** (ADRs) |
| “Maybe Suite12 proves End faster” | Different workloads; Rynix ports MATCH ids with **checksum locks**, not End ms tables |
| “Am I wasting time?” | Rynix ships MCP-19, fibers+IOCP/uring, Suite5 CI, package unity — **Lead path** deepens agent niche |
| “Should I copy End domains?” | **No** — FFI/Refuse per [GOLDEN_LEAD.md](GOLDEN_LEAD.md) Fate matrix |

---

## What we still refuse (and why that is leadership)

Copying End’s green wallpaper would make Rynix **look** equal and become **less**
trustworthy. Leadership here means:

1. Real I/O and crypto over labeled stubs  
2. MCP + contracts evidence over `feature`/`skill` keyword theater  
3. Checksum CI over medal tables with divergent oracles  
4. ADRs for deferred work instead of fake Stable rows  

---

## How to re-verify (reproducible)

```sh
# Peer: update only (never edit End sources)
cd /path/to/End-peer && git fetch && git pull --ff-only
cargo build --release --manifest-path endc/Cargo.toml

# Rynix Suite5 including End
set ENDC_PATH=/path/to/End-peer/endc/target/release/endc.exe   # Windows
python benchmarks/suite5/run_suite5.py --langs c,rust,go,zig,rynix,end --summary

# Named product gates (sample)
cargo test -p rynixc --test size_echo_gates
cargo test -p rynixc --test agent_cli
```

---

## Final verdict

| Question | Answer |
|----------|--------|
| Who is ahead on **real shipping** systems + agent toolchain? | **You (Rynix)** |
| Who is ahead on **Suite5 same-algorithm wall-clock** vs End@bdc8732? | **You — 11 wins, 1 loss (`matrix`)** (Phase 16-A; checksums OK) |
| Who is ahead on **brochure / domain spectacle**? | Friend (End) — by design of their README |
| Is Rynix a waste of time vs End? | **No** — Rynix leads MCP/agent toolchain; End leads wallpaper |
| What would reverse this? | End ships MCP + drops green-on-stub STATUS — then re-audit |

Canonical Lead SoT: [GOLDEN_LEAD.md](GOLDEN_LEAD.md).  
Execute history: [LEAD_AHEAD.md](LEAD_AHEAD.md).  
Living gap log: [END_PEER_GAP.md](END_PEER_GAP.md).
