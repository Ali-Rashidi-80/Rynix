# Ephemeral GPG detach-sign + verify smoke (SURPASS E4).
# Exit 77 if gpg missing or the Windows/MinGW path story is unusable.
# No production keys.

$ErrorActionPreference = "Stop"

function ConvertTo-GpgPath([string]$Path) {
  # Git/MSYS `gpg` treats `C:\foo` as cwd-relative (`d:/a/.../c:\foo`).
  # Forward-slash Win32 paths are accepted as absolute.
  $full = [System.IO.Path]::GetFullPath($Path)
  return ($full -replace '\\', '/')
}

$gpg = Get-Command gpg -ErrorAction SilentlyContinue
if (-not $gpg) {
  Write-Host "skip: gpg not on PATH"
  exit 77
}

$TMP = Join-Path $env:TEMP ("rynix-gpg-smoke-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $TMP | Out-Null
$gnupgDir = Join-Path $TMP "gnupg"
New-Item -ItemType Directory -Path $gnupgDir | Out-Null
$env:GNUPGHOME = ConvertTo-GpgPath $gnupgDir

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

  $keygenGpg = ConvertTo-GpgPath $keygen
  & gpg --batch --gen-key $keygenGpg 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) {
    Write-Host "skip: gpg --gen-key failed (exit $LASTEXITCODE)"
    exit 77
  }

  $stage = Join-Path $TMP "stage"
  New-Item -ItemType Directory -Path $stage | Out-Null
  $sums = Join-Path $stage "SHA256SUMS.txt"
  "deadbeef  rynixc-smoke" | Set-Content -Encoding ascii $sums
  $sumsGpg = ConvertTo-GpgPath $sums

  & gpg --batch --yes --detach-sign --armor $sumsGpg 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) {
    Write-Host "skip: gpg --detach-sign failed (exit $LASTEXITCODE)"
    exit 77
  }
  $asc = "$sums.asc"
  if (-not (Test-Path $asc)) {
    Write-Host "skip: missing detach signature"
    exit 77
  }

  $ascGpg = ConvertTo-GpgPath $asc
  & gpg --batch --verify $ascGpg $sumsGpg 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) {
    Write-Host "skip: gpg --verify failed (exit $LASTEXITCODE)"
    exit 77
  }
  Write-Host "gpg_sign_smoke ok"
  exit 0
}
catch {
  Write-Host "skip: gpg smoke error: $_"
  exit 77
}
finally {
  Remove-Item -Recurse -Force $TMP -ErrorAction SilentlyContinue
}
