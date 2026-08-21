# Rynix runtime ABI (`rynix_rt_*`)

Status: draft (Phase 7–8)

The compiler emits calls to these C symbols. `rynixc build` links the unity
translation unit [`rt/portable.c`](../rt/portable.c) (sources under
`rt/src/`). Phase 8 adds cooperative fibers and portable TCP; Linux
`--runtime=uring` enables a syscall io_uring path for read/write (with
portable fallback if ring setup fails).

## Conventions

- Pointers are opaque `ptr` / `void *`.
- Integers use LLVM `i64` / C `int64_t` unless noted.
- Region ids are small `i32` indices local to a compilation unit / process.
- Fiber stacks are 256 KiB with a leading guard page.

## Symbols

| Symbol | Signature | Role |
|--------|-----------|------|
| `rynix_rt_print` | `void(ptr)` | Print a NUL-terminated UTF-8 string + newline |
| `rynix_rt_panic` | `void(ptr)` | Abort with message on stderr |
| `rynix_rt_heap_alloc` | `ptr(i64)` | Zeroed heap allocation (`malloc`) |
| `rynix_rt_heap_free` | `void(ptr)` | Heap free (`free`) |
| `rynix_rt_region_create` | `void(i32)` | Ensure bump region `id` exists; reset cursor |
| `rynix_rt_region_reset` | `void(i32)` | Reset region cursor (reuse capacity) |
| `rynix_rt_region_alloc` | `ptr(i32, i64)` | Bump-allocate `size` bytes in region `id` |
| `rynix_rt_spawn` | `ptr(fn, ptr)` | Spawn a cooperative fiber |
| `rynix_rt_yield` | `void()` | Yield to the next ready fiber |
| `rynix_rt_sleep_ms` | `void(i64)` | Park at least `ms` milliseconds |
| `rynix_rt_run` | `void()` | Drain the local run queue until idle |
| `rynix_rt_fiber_count` | `i64()` | Live fibers (leak check) |
| `rynix_rt_read` | `i64(i64, ptr, i64)` | Colorless read (portable = blocking) |
| `rynix_rt_tcp_listen` | `i64(i64)` | Bind loopback TCP listen (non-blocking) |
| `rynix_rt_tcp_accept` | `i64(i64)` | Accept with yield loop |
| `rynix_rt_tcp_connect` | `i64(ptr, i64)` | Connect with yield loop |
| `rynix_rt_tcp_close` | `void(i64)` | Close socket |
| `rynix_rt_tcp_recv` / `send` | `i64(i64, ptr, i64)` | Fiber-safe socket I/O |
| `rynix_rt_vec_i64_*` / `map_i64_*` | (see header) | Region Vec/Map |
| `rynix_rt_uring_*` | Linux + `RYNIX_RT_URING` | Syscall io_uring read/write |

## Backends

| `--runtime` | Platform | Notes |
|-------------|----------|-------|
| `portable` (default) | All | Fibers + non-blocking TCP; blocking fd read/write |
| `uring` | Linux | `RYNIX_RT_URING`: syscall io_uring for read/write; TCP still non-blocking poll |

`rt/src/fiber_swap_x86_64.S` provides a SysV callee-saved swap for Linux
experiments; Windows uses the Microsoft fiber API instead.

## Placement mapping (Phase 6 → runtime)

| Escape | Placement | Codegen |
|--------|-----------|---------|
| `NoEscape` | stack | LLVM `alloca` |
| `ArgEscape` / `RegionEscape` | region | `rynix_rt_region_alloc` |
| `GlobalEscape` | heap | `rynix_rt_heap_alloc` + injected `rynix_rt_heap_free` |

## Linking

```sh
rynixc build hello.ryx -o hello --runtime=portable
# clang -O3 -flto=thin -ffunction-sections -fuse-ld=lld -I rt/include \
#       hello.ll rt/portable.c -o hello
```
