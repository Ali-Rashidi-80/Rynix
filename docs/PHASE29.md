# Phase 29 — Runtime / WASM / docs ceiling (Quality-10)

Parent: [GOLDEN_PATH.md](GOLDEN_PATH.md).

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | uring recv/send honesty (poll fallback documented) | `uring_recv_send_completion_smoke` | ✅ docs |
| B | TLS CI matrix | `tls_ci_matrix_documented` | ✅ |
| C | HTTP auth/method | `http_auth_or_method_gate` | ✅ deferral |
| D | Escape interprocedural limit | `escape_interproc_or_limit_doc` | ✅ docs |
| E | WASM host-import `env.print` (str) | `emit_wasm_host_print_str` | ✅ |
| F | Package UX + attest honesty | `package_ux_diag_gate` + `attest_docs_match_impl` | ✅ |
| G | Rynix Book skeleton (≥3 chapters) | `book_skeleton_exists` | ✅ |
| H | Suite5 post-P24 artifact links | `suite5_post_p24_artifact_links` | ✅ |
| I | Track C: RFC template | `rfc_or_contributing_sections` | ✅ |

Contract: [contracts/phase29_ceiling.contract.toml](contracts/phase29_ceiling.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| Documentation | 9.4 | 9.6 | Book skeleton + TLS/uring honesty docs |

## Out of scope

- nginx RPS bakeoff as Quality gate
- 1 GiB/s lexer requirement
- Full WASI
- CDN / Sigstore theater
- **Phase 30** remains **user-triggered only** (no auto push/tag/release from this phase).
