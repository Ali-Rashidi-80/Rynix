# io_uring TCP recv/send — Implemented (Phase 32)

**Gate:** `uring_tcp_recv_send_completion_smoke` (behavioral + this file).

## What ships

Under `--runtime=uring` on Linux (`RYNIX_RT_URING`):

| Op | Path |
|----|------|
| `accept` / `connect` | `io_uring` SQE when ready |
| File `read` / `write` | `rynix_rt_uring_read` / `_write` |
| TCP `recv` / `send` | Prefer `rynix_rt_uring_read` / `_write` (IORING_OP_READ/WRITE on sockets) when `rynix_rt_uring_ready()`; else poll/yield fallback |

## Evidence

- `rt/src/net.c` — uring branch before poll loop
- CI: uring TCP echo / load smokes under `sanitizer-rt` / uring steps
- Soft gate in agent_cli asserts net.c contains uring_ready + uring_read/write
