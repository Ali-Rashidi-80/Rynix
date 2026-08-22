# Rynix runtime ABI (`rynix_rt_*`)

Status: shipping (Phase 7–8)

The compiler emits calls to these C symbols. `rynixc build` links the unity
translation unit [`rt/portable.c`](../rt/portable.c) (sources under
`rt/src/`). Linux `--runtime=uring` enables a **fiber-aware** syscall io_uring
path: submit SQE → park fiber → harvest CQEs in `rynix_rt_run` (blocking
`enter(min_complete=1)` only when the ready queue is empty and fibers are
parked). `tcp_accept` / `tcp_connect` use uring when the ring is ready.

## Conventions

- Pointers are opaque `ptr` / `void *`.
- Integers use LLVM `i64` / C `int64_t` unless noted.
- Region ids are small `i32` indices local to a compilation unit / process.
- Fiber stacks are 256 KiB with a leading guard page.

## Symbols

| Symbol | Signature | Role |
|--------|-----------|------|
| `rynix_rt_print` | `void(ptr)` | Print a NUL-terminated UTF-8 string + newline |
| `rynix_rt_print_i64` | `void(i64)` | Print a decimal i64 + newline (benches / agents) |
| `rynix_rt_panic` | `void(ptr)` | Abort with message on stderr |
| `rynix_rt_heap_alloc` | `ptr(i64)` | Zeroed heap allocation (`malloc`) |
| `rynix_rt_heap_free` | `void(ptr)` | Heap free (`free`) |
| `rynix_rt_region_create` | `void(i32)` | Ensure bump region `id` exists; reset cursor |
| `rynix_rt_region_reset` | `void(i32)` | Reset region cursor (reuse capacity) |
| `rynix_rt_region_alloc` | `ptr(i32, i64)` | Bump-allocate `size` bytes in region `id` |
| `rynix_rt_spawn` | `ptr(fn, ptr)` | Spawn a cooperative fiber |
| `rynix_rt_yield` | `void()` | Yield to the next ready fiber |
| `rynix_rt_fiber_park` / `unpark` | — | Park/wake for I/O waits |
| `rynix_rt_sleep_ms` | `void(i64)` | Park at least `ms` milliseconds |
| `rynix_rt_run` | `void()` | Drain ready queue + uring CQ until idle |
| `rynix_rt_fiber_count` | `i64()` | Live fibers (leak check) |
| `rynix_rt_read` / `write` | `i64(...)` | Colorless I/O (uring when ready) |
| `rynix_rt_tcp_*` | (see header) | Fiber-safe TCP; uring accept/connect when ready |
| `rynix_rt_json_get_i64` | `i64(json, key)` | Parse minimal JSON object int field |
| `rynix_rt_http_get_json_i64` | `i64(host, port, path, field)` | HTTP GET + JSON field (soft std) |
| `rynix_rt_vec_i64_*` / `map_i64_*` | (see header) | Region Vec/Map |
| `rynix_rt_uring_*` | Linux + `RYNIX_RT_URING` | Fiber-aware SQE read/write/accept/connect + poll/wait |

## Backends

| `--runtime` | Platform | Notes |
|-------------|----------|-------|
| `portable` (default) | All | Fibers + non-blocking TCP; blocking fd read/write |
| `uring` | Linux | Fiber-aware io_uring for read/write/accept/connect |

## Placement mapping (Phase 6 → runtime)

| Escape | Placement | Codegen |
|--------|-----------|---------|
| `NoEscape` | stack | LLVM `alloca` |
| `ArgEscape` / `RegionEscape` | region | `rynix_rt_region_alloc` |
| `GlobalEscape` | heap | `rynix_rt_heap_alloc` + injected `rynix_rt_heap_free` |

## Linking

```sh
rynixc build hello.ryx -o hello --runtime=portable
# Linux:
# rynixc build hello.ryx -o hello --runtime=uring
```
