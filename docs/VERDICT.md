# Verdict — Rynix vs End (who is ahead?)

**Date:** 2026-08-25  
**Peer:** [IrMaho/End](https://github.com/IrMaho/End) `main` @ **`cf5bef3`**  
(local clone `D:\0\End-peer`, `git fetch` + `pull --ff-only` — **no peer source edits**)  
**This repo:** Rynix Phase 14 complete ([PHASE14.md](PHASE14.md)); Phase 15
Wave A complete ([PHASE15.md](PHASE15.md)). Phase 12 product realness:
[LEAD_AHEAD.md](LEAD_AHEAD.md).

This document answers one question under audit rules:

> Is continuing Rynix a waste of time relative to End, or is Rynix already
> ahead on what actually ships?

---

## One-line answer

**Rynix is ahead on the shipping systems + agent core that End actually
implements in working code.** End is ahead only on **spectacle** (README domain
wallpaper, ~60 CLI names, C11/UI narrative, suite12 marketing) — surfaces we
refuse to fake ([ADR-0007](adr/0007-deferred-ui-frameworks.md),
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

| Axis | End @ cf5bef3 | Rynix | Winner |
|------|---------------|-------|--------|
| Native binaries | **Real** C11 → host CC | **Real** LLVM IR → clang ThinLTO | **Rynix** (AOT product + size gates; C11 deferred by choice) |
| Async / I/O runtime | Thin TCP; fiber theater | Fibers + **portable / uring / IOCP** | **Rynix** |
| TLS | Plaintext TCP + cipher **label** | Real SChannel / OpenSSL | **Rynix** |
| Crypto | SHA-256 + HMAC real; Argon2/AES claims fake | SHA + HMAC + AES-GCM KATs | **Rynix** |
| HTTP | Thin string helpers on TCP | One-shot + **bounded loop** + JSON | **Rynix** |
| WebSocket | — / incomplete | RFC 6455 + wire smokes | **Rynix** |
| Agent CLI | ~60 names, uneven depth | graph/slice/impact/eval/patch/verify… | Tie on overlap; End broader **names** |
| **MCP** | **Absent** | **18 tools** (`mcp-serve`) | **Rynix** |
| Packages | Local path; **PubGrub theater** | Path + local index (scan + sparse) + unity/semver/lock/workspace + **local digest attest** | **Rynix** |
| WASM | Text / toy surface | `emit-ll --target=wasm32` + **`emit-wasm`** (real `\0asm`) + Node run gate (Phase 15) — **no WASI** | **Rynix** (honest subset) |
| Memory / escape | Real region/borrow subset | Escape + move + `#^ effect: pure` + explain-alloc | **Rynix** (transparency) |
| Editor LSP | LSP server; VS Code **without** LanguageClient | `lsp-serve` + **LanguageClient** + CodeLens + workspace goto | **Rynix** |
| Microbench fairness | suite12 (different programs; checksum caveats) | Suite5 **C↔Rynix CI** + optional End slot | **Rynix** |
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

### vs End on Suite5 (2026-08-25, this machine)

Peer `endc` built release-only from untouched `D:\0\End-peer` @ `cf5bef3`
(`ENDC_PATH=…/endc/target/release/endc.exe`).

**Peer compiler regressions discovered during this audit (not Rynix edits):**

1. Statement `if cond { … }` **fails to parse** — End’s own
   `benchmarks/suite12/suite12_end.end` errors with `Expected token Else, found LBrace`.
2. Expression `a if cond else b` **always evaluates to 0** in probed binaries.
3. Rynix-owned Suite5 `.end` ports use only `for` / `while` / `getenv as i64`
   (see `benchmarks/suite5/regen_end_ports.py` + `END_INTEGRATION.md`).

Live head-to-head numbers: `benchmarks/suite5/suite5_summary_2026-08-25_vs_end_mulhu.txt`
(and table in [END_PEER_GAP.md](END_PEER_GAP.md) §2). Checksums must match C on every
counted End row.

Binary size (hello full RT gate): **&lt;300 KiB** (~13 KiB measured 2026-08-25).
Suite5 `--bench` Rynix bins were smaller than End `--strip` on prior Windows run.

---

## Doubt checklist (anxiety → evidence)

| Worry | Evidence response |
|-------|-------------------|
| “End README looks finished; maybe I’m late” | ~62 docs + green domains; TLS/Cranelift/PubGrub/GGUF/Raft are stubs in-tree |
| “End has more CLI commands” | Volume ≠ depth; Rynix has **MCP** (End has zero) |
| “End has C11 / UI / frameworks” | Real C transpile exists; UI/registry/JIT largely theater. We deferred C11/UI **on purpose** (ADRs) |
| “Maybe Suite12 proves End faster” | Different workloads; Rynix ports MATCH ids with **checksum locks**, not End ms tables |
| “Am I wasting time?” | Rynix ships real TLS, fibers+IOCP/uring, MCP-18, Suite5 CI, package unity — End cannot match those with parse tests |
| “Should I copy End domains?” | **No** — that wastes months ([LEAD_AHEAD.md](LEAD_AHEAD.md) §0b) |

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
| Who is ahead on **Suite5 same-algorithm wall-clock** vs End@cf5bef3? | **You — 10 wins, 2 losses (`nested`, `sum`)** (checksums OK; `sum` gap ~1.03×) |
| Who is ahead on **brochure / domain spectacle**? | Friend (End) — by design of their README |
| Is Rynix a waste of time vs End? | **No** — Rynix leads where code must be true; End’s suite12 `.end` does not even parse at peer HEAD |
| What would reverse this? | End ships real TLS, MCP, fixes `if` parsing, and drops STATUS green on stubs — then re-audit |

Canonical execute history: [LEAD_AHEAD.md](LEAD_AHEAD.md).  
Living gap log: [END_PEER_GAP.md](END_PEER_GAP.md).
