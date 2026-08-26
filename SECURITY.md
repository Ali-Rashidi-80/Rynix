# Security policy

**Languages:** [English](SECURITY.md) (default) · [فارسی](SECURITY.fa.md)

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

## Threat model

See [docs/SECURITY_THREAT_MODEL.md](docs/SECURITY_THREAT_MODEL.md) (STRIDE summary).

## Scope notes

- **`rynixc mcp-serve`** reads arbitrary source from the MCP client and writes diagnostics/fixes — run only with trusted clients.
- **Runtime** (`rt/`) is C with ASan coverage in CI; production deployments should enable sanitizers during development.
- **Compiler** defaults to host clang link (`--sandbox=none`); opt-in `--sandbox=docker` isolates the link step when Docker is available ([ADR-0022](docs/adr/0022-build-sandbox.md)). Treat untrusted `.ryx` like untrusted C.

## Secure development

- CI runs `cargo test --workspace` on Ubuntu and Windows.
- Runtime fiber/TCP/load paths run under ASan on Ubuntu CI.
- Suite5 checksum gate on Ubuntu CI prevents silent benchmark drift.
