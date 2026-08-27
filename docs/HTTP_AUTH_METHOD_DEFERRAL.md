# HTTP auth / method — Bearer soft Implemented (Phase 32)

**Former status:** Deferred (Phase 29). **Now:** soft Bearer header equality.

## Soft surface

`http_serve_loop_bearer_json_i64(port, path, expected_token, max_reqs)`

- Requires `Authorization: Bearer <token>` with exact opaque token match
- On success: `200 {"value": 1}`
- Path match + bad auth: `401`
- Else: `404`

Gate: `http_bearer_header_soft_gate` / `http_bearer_smoke`.

Still out: full auth middleware, HTTP/2, method matrix beyond soft GET/POST product path.
