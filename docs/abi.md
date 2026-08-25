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
| `rynix_rt_json_has_i64` | `i64(json, key)` | 1 if int field present, else 0 |
| `rynix_rt_http_get_json_i64` | `i64(host, port, path, field)` | HTTP GET + JSON field (soft std) |
| `rynix_rt_http_post_json_i64` | `i64(host, port, path, body, field)` | HTTP POST JSON + response field |
| `rynix_rt_http_serve_once_json_i64` | `i64(port, path, value)` | One-shot HTTP JSON server (soft std) |
| `rynix_rt_http_serve_once_echo_json_i64` | `i64(port, path, field)` | One-shot echo request JSON field |
| `rynix_rt_http_serve_loop_json_i64` | `i64(port, path, value, max_reqs)` | Bounded loop: exactly `max_reqs` matching GETs → `0`; `max_reqs <= 0` → `-1` |
| `rynix_rt_http_serve_loop_2paths_json_i64` | `i64(port, path_a, val_a, path_b, val_b, max_reqs)` | Dual-path: matching GET on either path counts toward `max_reqs` |
| `rynix_rt_http_serve_loop_3paths_json_i64` | `i64(port, path_a, val_a, path_b, val_b, path_c, val_c, max_reqs)` | Triple-path: matching GET on any listed path counts toward `max_reqs` |
| `rynix_rt_http_serve_loop_path_param_json_i64` | `i64(port, prefix, max_reqs)` | Path-param: GET `{prefix}{digits}` echoes parsed i64 |
| `rynix_rt_http_serve_loop_header_json_i64` | `i64(port, path, header, max_reqs)` | Header: GET + decimal request header → JSON value |
| `rynix_rt_http_serve_loop_post_echo_json_i64` | `i64(port, path, field, max_reqs, max_body)` | POST echo; body longer than `max_body` → 400 |
| `rynix_rt_http_serve_loop_keepalive_json_i64` | `i64(port, path, value, max_reqs)` | One accept; keep-alive GETs until `max_reqs` |
| `rynix_rt_http_tls_serve_once_json_i64` / `_get_json_i64` | HTTP+TLS JSON | Same backends as TLS echo; stub `-2` |
| `rynix_rt_frame_serve_once_echo` / `_client_echo` | framed TCP echo | Length-prefixed binary framing |
| `rynix_rt_ws_accept_key_eq` / `_accept_sha1_first_i64` | WS accept | RFC 6455 Sec-WebSocket-Accept |
| `rynix_rt_ws_frame_encode` / `_decode` / `_roundtrip_ok` | WS frames | 7/16/64-bit lengths; mask XOR; fragmentation KATs |
| `rynix_rt_ws_message_decode` | WS reassembly | Fragmented message decode (cap `RYNIX_WS_MAX_PAYLOAD`) |
| `rynix_rt_ws_serve_once_echo` / `_client_echo` | WS upgrade echo | HTTP 101 + one text frame (short) |
| `rynix_rt_ws_serve_once_echo_n` / `_client_echo_n` | WS large echo | 16/64-bit length on wire (≤1 MiB) |
| `rynix_rt_tls_serve_once_echo` / `_client_echo` | TLS echo | SChannel (Win); OpenSSL if `-DRYNIX_RT_OPENSSL`; else `-2` |
| `rynix_rt_sha256_first_i64` | `i64(data)` | SHA-256 first 8 bytes BE (soft std) |
| `rynix_rt_hmac_sha256_first_i64` | `i64(key, data)` | HMAC-SHA256 first 8 bytes BE (RFC 4231) |
| `rynix_rt_fs_write_file` | `i64(path, data)` | Whole-file write (`0` / `-1`) |
| `rynix_rt_fs_read_file` | `ptr(path)` | Heap NUL string or NULL |
| `rynix_rt_fs_read_file_eq` | `i64(path, expect)` | Compare file to string (`0` / `-1`) |
| `rynix_rt_fs_exists` | `i64(path)` | `1` if fopen succeeds, else `0` |
| `rynix_rt_fs_remove_file` | `i64(path)` | Unlink (`0`; missing → `0`) |
| `rynix_rt_aes128_gcm_nist_empty_tag_first_i64` | `i64()` | AES-GCM NIST empty tag (BCrypt/OpenSSL) |
| `rynix_rt_kv_new` / `_put` / `_get` / `_len` | region string→i64 map | Arena KV (soft std) |
| `rynix_rt_vec_i64_*` / `map_i64_*` | (see header) | Region Vec/Map |
| `rynix_rt_uring_*` | Linux + `RYNIX_RT_URING` | Fiber-aware SQE read/write/accept/connect + poll/wait |
| `rynix_rt_iocp_*` | Windows + `RYNIX_RT_IOCP` | associate/recv/send/**accept**/**connect** + poll/wait |

## Backends

| `--runtime` | Platform | Notes |
|-------------|----------|-------|
| `portable` (default) | All | Fibers + non-blocking TCP; blocking fd read/write |
| `uring` | Linux | Fiber-aware io_uring for read/write/accept/connect |
| `iocp` | Windows | Fiber-aware IOCP: AcceptEx/ConnectEx + WSARecv/WSASend |

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
# Windows:
# rynixc build hello.ryx -o hello --runtime=iocp
```
