# io_uring recv/send — honesty limit (Phase 29-A)

**Gate:** `uring_recv_send_completion_smoke` (this file + named cargo test).

## What ships

Under `--runtime=uring` on Linux:

| Op | Path |
|----|------|
| `accept` / `connect` | Prefer `io_uring` SQE when `rynix_rt_uring_ready()` |
| File `read` / `write` | Prefer uring via `rynix_rt_uring_read` / `_write`, else portable |
| TCP `recv` / `send` | **Poll / yield fallback** — nonblocking `recv`/`send` in a yield loop (`rt/src/net.c`) |

## Limit (honest)

TCP **recv/send do not submit uring SQEs** today. Reducing that poll fallback to
true completion-path recv/send remains future work (Linux CI). This phase
documents the limit rather than claiming completion-path TCP I/O.

## Evidence

- `rt/src/uring.c` — accept/connect/read/write SQEs
- `rt/src/net.c` — `rynix_rt_tcp_recv` / `_send` yield+recv/send loop
- Existing smoke: `uring_sqe_smoke_c` (not a full recv/send completion gate)
