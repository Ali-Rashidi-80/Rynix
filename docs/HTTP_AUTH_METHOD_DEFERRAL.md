# HTTP auth / method — deferral (Phase 29-C)

**Gate:** `http_auth_or_method_gate` (accepts this deferral file).

## Status

**Deferred** as a dedicated soft surface.

## What already works

- Bounded header loop soft: `http_serve_loop_header_json_i64(port, path, header_name, max_reqs)`
  accepts any header name (including `Authorization`) whose value is a **decimal i64**.
- One-shot GET/POST JSON softs; path_param / keepalive / body-bounded loops.

## What is deferred

- Bearer / opaque token auth (non-decimal header values)
- Arbitrary HTTP methods beyond the existing GET/POST softs
- Middleware-style auth as language surface

Revisit with a named soft + RT smoke before marking this wave Implemented.
