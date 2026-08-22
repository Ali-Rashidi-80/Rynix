# Security policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | yes (best effort) |

## Reporting a vulnerability

Please report security issues **privately** (do not open a public issue with exploit details).

Include:

- Affected component (`rynixc`, `rt/`, MCP server, etc.)
- Reproduction steps or proof-of-concept
- Impact assessment

We aim to acknowledge reports within 7 days.

## Scope notes

- **`rynixc mcp-serve`** reads arbitrary source from the MCP client and writes diagnostics/fixes — run only with trusted clients.
- **Runtime** (`rt/`) is C with ASan coverage in CI; production deployments should enable sanitizers during development.
- **Compiler** does not sandbox build subprocesses: `build`/`run` invoke `clang` on generated IR — treat untrusted `.ryx` like untrusted C.

## Secure development

- CI runs `cargo test --workspace` on Ubuntu and Windows.
- Runtime fiber/TCP/load paths run under ASan on Ubuntu CI.
- Suite5 checksum gate on Ubuntu CI prevents silent benchmark drift.
