#!/usr/bin/env bash
# Ephemeral GPG detach-sign + verify smoke (SURPASS E4 evidence).
# Skips with exit 77 if `gpg` is missing. Does not use production keys.

set -euo pipefail

if ! command -v gpg >/dev/null 2>&1; then
  echo "skip: gpg not on PATH"
  exit 77
fi

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

export GNUPGHOME="$TMP/gnupg"
mkdir -m 700 "$GNUPGHOME"

# Batch key without passphrase for CI/local smoke.
cat > "$TMP/keygen" <<'EOF'
%no-protection
Key-Type: RSA
Key-Length: 2048
Name-Real: Rynix Smoke
Name-Email: smoke@rynix.invalid
Expire-Date: 0
%commit
EOF

gpg --batch --gen-key "$TMP/keygen" >/dev/null 2>&1

STAGE="$TMP/stage"
mkdir -p "$STAGE"
echo "deadbeef  rynixc-smoke" > "$STAGE/SHA256SUMS.txt"

gpg --batch --yes --detach-sign --armor "$STAGE/SHA256SUMS.txt"
test -f "$STAGE/SHA256SUMS.txt.asc"

gpg --batch --verify "$STAGE/SHA256SUMS.txt.asc" "$STAGE/SHA256SUMS.txt" >/dev/null 2>&1
echo "gpg_sign_smoke ok"
