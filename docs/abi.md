# Rynix runtime ABI (`rynix_rt_*`)

Status: draft (Phase 7)

The compiler emits calls to these C symbols. Phase 7 links the portable
implementation in [`rt/portable.c`](../rt/portable.c). Phase 8 replaces the
I/O and scheduling subset with a fiber + `io_uring` backend on Linux while
keeping the same symbol names.

## Conventions

- Pointers are opaque `ptr` / `void *`.
- Integers use LLVM `i64` / C `int64_t` unless noted.
- Region ids are small `i32` indices local to a compilation unit / process.

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

## Placement mapping (Phase 6 → runtime)

| Escape | Placement | Codegen |
|--------|-----------|---------|
| `NoEscape` | stack | LLVM `alloca` |
| `ArgEscape` / `RegionEscape` | region | `rynix_rt_region_alloc` |
| `GlobalEscape` | heap | `rynix_rt_heap_alloc` (+ later `free`) |

## Linking

```sh
rynixc build hello.ryx -o hello
# expands to approximately:
#   rynixc emit-ll hello.ryx -o hello.ll
#   clang -O3 -flto=thin -ffunction-sections -Wl,--gc-sections \
#         hello.ll rt/portable.c -o hello
```

On Windows, `clang` from LLVM or WinLibs is required at **run** time for
`rynixc build`, not at compiler build time (ADR-0005).
