# Bake-off / RPS methodology (in-tree)

Status: complete for the shipping runtime acceptance gate.

## What we measure

Local loopback TCP echo RPS using cooperative fibers:

| Harness | Source | Gate |
|---------|--------|------|
| TCP echo RPS | `rt/tests/tcp_echo_rps.c` | ≥1 RPS (smoke) |
| Load harness | `rt/tests/load_harness.c` | ≥1 RPS over 64 iters |
| IOCP echo (Win) | `rt/tests/iocp_echo_smoke.c` | `-DRYNIX_RT_IOCP` |

Run (with clang + `rt/portable.c`):

```sh
clang -O1 -I rt/include rt/portable.c rt/tests/load_harness.c -o target/load_harness
./target/load_harness
```

On Linux with uring:

```sh
clang -O1 -DRYNIX_RT_URING -I rt/include rt/portable.c rt/tests/load_harness.c -o target/load_uring
./target/load_uring
```

On Windows with IOCP:

```sh
clang -O1 -DRYNIX_RT_IOCP -I rt/include rt/portable.c rt/tests/iocp_echo_smoke.c \
  -lws2_32 -o target/iocp_echo
./target/iocp_echo
```

Or `rynixc build … --runtime=iocp` (defines `RYNIX_RT_IOCP`).

## Windows async I/O quality (SURPASS D2)

| Mode | Mechanism | Notes |
|------|-----------|-------|
| `--runtime=portable` (default) | Non-blocking Winsock + fiber `yield` polls | Correct; more wakeups under load |
| `--runtime=iocp` | IOCP + `WSARecv`/`WSASend` + **AcceptEx/ConnectEx** | Real completion-port accept/connect/data |
| Linux `--runtime=uring` | io_uring SQE + park | Lowest syscall overhead on Linux |

Do **not** claim portable poll matches IOCP or uring latency. Local `load_harness`
RPS on Windows remains a valid smoke (≥1 RPS gate); publish side-by-side
numbers with the OS and `--runtime` named.

## Cross-runtime comparison (optional)

If `go` is installed, `scripts/bakeoff_go_echo.go` provides a comparable
loopback echo client/server RPS printout using the same iter count. Numbers
are machine-local; they are not marketing claims. Record both outputs side by
side when publishing a blog post — the compiler gate only requires the
in-tree harness to pass.

```sh
go run scripts/bakeoff_go_echo.go
```

For CPU microbenchmarks across Rynix/C/Rust/Go/Zig, use
[benchmarks/suite5](../benchmarks/suite5/README.md) (**12 workloads**, checksum gate).
