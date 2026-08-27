# TLS CI matrix (Phase 29-B → Phase 32 assert)

**Gate:** `tls_linux_ci_row_green` (asserts CI job name + Linux TLS smoke).

| Platform | Backend | CI job | Notes |
|----------|---------|--------|-------|
| Windows | SChannel | `size-gate` / `test` (windows-latest) | `http_tls_product_smoke` |
| Linux | OpenSSL (`libssl`) | `size-gate` companion + Ubuntu `test` | Link when OpenSSL present; documented skip otherwise |
| macOS | — | Not required | Deferred |

## Honesty

- Soft HTTP-over-TLS is real SChannel / OpenSSL — not a cipher-label stub.
- CI may skip when host lacks TLS libs; that is documented skip, not theater.
- Job name cited for Phase 32: `http_tls_product_smoke` in `size_echo_gates`.

## Pointers

- Runtime: `rt/src/tls.c`
- Gate: `http_tls_product_smoke` in `crates/rynixc/tests/size_echo_gates.rs`
