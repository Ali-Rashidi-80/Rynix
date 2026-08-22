# Bake-off / RPS methodology (in-tree)

Status: complete for the shipping runtime acceptance gate.

## What we measure

Local loopback TCP echo RPS using cooperative fibers:

| Harness | Source | Gate |
|---------|--------|------|
| TCP echo RPS | `rt/tests/tcp_echo_rps.c` | ≥1 RPS (smoke) |
| Load harness | `rt/tests/load_harness.c` | ≥1 RPS over 64 iters |

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
