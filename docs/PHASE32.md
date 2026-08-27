# Phase 32 — Runtime / HTTP close (behavioral)

Parent: [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md).

Supersedes Phase 29 honesty/doc gates for uring TCP, HTTP auth, escape, TLS matrix assert.

## Waves

| Wave | Theme | Gate | Status |
|------|--------|------|--------|
| A | uring TCP recv/send via read/write SQE | `uring_tcp_recv_send_completion_smoke` | ✅ |
| B | Bearer header soft | `http_bearer_header_soft_gate` | ✅ |
| C | TLS Linux CI row assert | `tls_linux_ci_row_green` | ✅ |
| D | Escape SCC mutual-recursion test | `escape_interproc_improvement_gate` | ✅ |
| E | Contract `phase32_runtime_close` | `verify_phase32_runtime_close_contract` | ✅ |

Contract: [contracts/phase32_runtime_close.contract.toml](contracts/phase32_runtime_close.contract.toml).

## Axis note

| Axis | Prior | After | Gates |
|------|------:|------:|-------|
| C runtime quality | 9.2 | 9.4 | uring TCP completion path |
| Security | 9.3 | 9.4 | Bearer soft (auth surface) |

## Out of scope

- nginx RPS; HTTP/2; full auth middleware
