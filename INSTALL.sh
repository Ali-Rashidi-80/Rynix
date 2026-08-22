#!/usr/bin/env bash
# Install rynixc from this repository (Unix).
set -euo pipefail
cd "$(dirname "$0")"

echo "==> Building and installing rynixc"
cargo install --path crates/rynixc --force

if command -v rynixc >/dev/null 2>&1; then
  echo "Installed: $(command -v rynixc)"
  rynixc --version
else
  echo "rynixc installed, but not on PATH yet. Add:"
  echo "  \$HOME/.cargo/bin"
fi

echo ""
echo "Try:  rynixc run examples/01_hello.ryx"
echo "      rynixc arch check"
echo "Docs: INSTALL.md  README.md"
