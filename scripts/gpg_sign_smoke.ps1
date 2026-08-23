# Ephemeral GPG detach-sign + verify smoke (SURPASS E4).
# Exit 77 if gpg missing. No production keys.

$ErrorActionPreference = "Stop"
$gpg = Get-Command gpg -ErrorAction SilentlyContinue
if (-not $gpg) {
  Write-Host "skip: gpg not on PATH"
  exit 77
}

$TMP = Join-Path $env:TEMP ("rynix-gpg-smoke-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $TMP | Out-Null
$env:GNUPGHOME = Join-Path $TMP "gnupg"
New-Item -ItemType Directory -Path $env:GNUPGHOME | Out-Null

try {
  $keygen = Join-Path $TMP "keygen"
  @"
%no-protection
Key-Type: RSA
Key-Length: 2048
Name-Real: Rynix Smoke
Name-Email: smoke@rynix.invalid
Expire-Date: 0
%commit
"@ | Set-Content -Encoding ascii $keygen

  & gpg --batch --gen-key $keygen 2>$null | Out-Null

  $stage = Join-Path $TMP "stage"
  New-Item -ItemType Directory -Path $stage | Out-Null
  $sums = Join-Path $stage "SHA256SUMS.txt"
  "deadbeef  rynixc-smoke" | Set-Content -Encoding ascii $sums

  & gpg --batch --yes --detach-sign --armor $sums
  $asc = "$sums.asc"
  if (-not (Test-Path $asc)) { throw "missing $asc" }

  & gpg --batch --verify $asc $sums 2>$null | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "gpg --verify failed" }
  Write-Host "gpg_sign_smoke ok"
  exit 0
}
finally {
  Remove-Item -Recurse -Force $TMP -ErrorAction SilentlyContinue
}
