# TLS CI matrix (Phase 29-B)

**Gate:** `tls_ci_matrix_documented`.

| Platform | Backend | CI | Notes |
|----------|---------|-----|-------|
| Windows | SChannel | GitHub Actions Windows job | Product smoke via `http_tls_product_smoke` when clang/RT available |
| Linux | OpenSSL (`libssl`) | Ubuntu job | Link against system OpenSSL when present; skip if headers/libs missing |
| macOS | — | Not required for Quality-10 | Deferred; portable/TLS optional |

## Honesty

- Soft HTTP-over-TLS is **real** SChannel / OpenSSL — not a cipher-label stub.
- CI may **skip** TLS smokes when the host lacks TLS libs; that is documented skip,
  not a green theater pass.
- Full cross-distro OpenSSL matrix (versions × FIPS) is out of Quality-10 scope.

## Pointers

- Runtime: `rt/src/tls.c`
- Gate: `http_tls_product_smoke` in `crates/rynixc/tests/size_echo_gates.rs`
- Niche-10: [NICHE10.md](NICHE10.md)
