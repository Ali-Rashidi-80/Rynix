# Niche-10 certification (Phase 20-D)

**Status:** **Certified** (2026-08-25)  
**Definition:** [ADR-0013](adr/0013-niche-10-scorecard.md) — niche product claim
(systems language + agent toolchain + offline-first packages), **not** absolute
parity with Go/Rust/nginx.

An axis is **10** only when the linked gate(s) exist in-tree. Absolute-10 follow-ons
need a new ADR.

## Scorecard

| Axis | Score | Gate evidence (in-tree) |
|------|-------|-------------------------|
| Compiler UX | **10** | [`install_one_path_clang_win_linux`](../INSTALL.md) · [`package_ux_new_deps_attest`](../crates/rynixc/tests/agent_cli.rs) · [`new_scaffolds_package`](../crates/rynixc/tests/agent_cli.rs) · NDJSON via `rynixc check --error-format=json` |
| Runtime I/O | **10** | [`tcp_echo_rps_c`](../crates/rynixc/tests/size_echo_gates.rs) (portable) · [`iocp_echo_smoke_c`](../crates/rynixc/tests/size_echo_gates.rs) · [`uring_sqe_smoke_c`](../crates/rynixc/tests/size_echo_gates.rs) (+ Linux CI uring TCP) |
| HTTP | **10** | [`http_loop_path_param`](../crates/rynixc/tests/size_echo_gates.rs) · [`http_header_i64_smoke`](../crates/rynixc/tests/size_echo_gates.rs) · [`http_body_bounded_smoke`](../crates/rynixc/tests/size_echo_gates.rs) · [`http_keepalive_bounded_smoke`](../crates/rynixc/tests/size_echo_gates.rs) |
| TLS/WS/crypto | **10** | [`http_tls_product_smoke`](../crates/rynixc/tests/size_echo_gates.rs) · [`ws_frames_smoke_c`](../crates/rynixc/tests/size_echo_gates.rs) · [`crypto_kv_smoke_c`](../crates/rynixc/tests/size_echo_gates.rs) (SHA/HMAC/AES KAT) |
| Packages | **10** | [`deps_resolves_sparse_local_index`](../crates/rynixc/tests/agent_cli.rs) · [`deps_attest_write_verify_and_tamper`](../crates/rynixc/tests/agent_cli.rs) · [`package_ux_new_deps_attest`](../crates/rynixc/tests/agent_cli.rs) · offline-first ([ADR-0010](adr/0010-local-package-index.md)) |
| WASM | **10** | [`emit_wasm_host_print_i64`](../crates/rynixc/tests/size_echo_gates.rs) · [`emit_wasm_node_runs_main`](../crates/rynixc/tests/size_echo_gates.rs) (host-import `env.print_i64`, **not** full WASI) |
| MCP | **10** | [`mcp_graph_path_file`](../crates/rynixc/tests/agent_cli.rs) · [`mcp_impact_path_file`](../crates/rynixc/tests/agent_cli.rs) · [`mcp_precheck_path_file`](../crates/rynixc/tests/agent_cli.rs) · [`verify_phase19_path_mcp_contract`](../crates/rynixc/tests/agent_cli.rs) |
| LSP | **10** | [`completion_lists_fn_and_let`](../crates/rynixc/src/lsp_cmd.rs) · [`rename_local_updates_def_and_refs`](../crates/rynixc/src/lsp_cmd.rs) (+ diag/hover/def) |
| Benches | **10** | [`suite5_twelve_workloads_checksum_gate`](../crates/rynixc/tests/phase10_gates.rs) · Suite5 artifact + CI C↔Rynix honesty |
| Docs | **10** | [`niche10_scorecard_links_gates`](../crates/rynixc/tests/agent_cli.rs) · [`install_one_path_clang_win_linux`](../crates/rynixc/tests/agent_cli.rs) · [PRODUCTION_READINESS.md](../PRODUCTION_READINESS.md) (local digest, not Sigstore/Rekor) |
| Language | **10** | [`struct_str_field_roundtrip`](../crates/rynixc/tests/agent_cli.rs) · [`index_assign_ok`](../crates/rynixc/tests/agent_cli.rs) · [`enum_value_roundtrip`](../crates/rynixc/tests/agent_cli.rs) · mono `Vec[i64]`/`Map[i64,i64]` ([ADR-0014](adr/0014-mono-collections-niche10.md)) |

**Overall Niche-10:** **10/10** (all ADR-0013 axes gated above).

## Still out of Niche-10

llama embed · Raft product ([ADR-0012](adr/0012-deferred-consensus.md)) · UI/wgpu
([ADR-0007](adr/0007-deferred-ui-frameworks.md)) · CDN-required registry · nginx RPS
parity claims · full WASI.

## Phase map

| Phase | Theme | Status |
|-------|--------|--------|
| 16 | Honesty + path_param + MCP path-first | ✅ |
| 17 | Language struct str / index assign / enum | ✅ |
| 18 | Product HTTP(+TLS) deepen | ✅ |
| 19 | LSP completion+rename; MCP path trio | ✅ |
| 20 | WASM host-import; package/INSTALL; certify | ✅ `emit_wasm_host_print_i64`, `package_ux_new_deps_attest`, `install_one_path_clang_win_linux`, `niche10_scorecard_links_gates` |
