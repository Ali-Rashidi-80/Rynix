# Rynix runtime ABI (`rynix_rt_*`)

Status: draft (Phase 7–8)

The compiler emits calls to these C symbols. `rynixc build` links the unity
translation unit [`rt/portable.c`](../rt/portable.c) (sources under
`rt/src/`). Phase 8 adds cooperative fibers; Linux `--runtime=uring` enables
io_uring stubs (`RYNIX_RT_URING`) while keeping the same symbol names.

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
| `rynix_rt_write` | `i64(i64, ptr, i64)` | Colorless write (portable = blocking) |
| `rynix_rt_now_ms` | `i64()` | Monotonic-ish milliseconds |

## Backends

| `--runtime` | Platform | Notes |
|-------------|----------|-------|
| `portable` (default) | All | Win32 Fibers / POSIX `ucontext`; blocking I/O |
| `uring` | Linux | Defines `RYNIX_RT_URING`; full SQE park TBD + liburing |

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
