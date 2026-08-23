# Build a local release staging dir + SHA256SUMS (SURPASS E4 packaging).
# Optional GPG: set RYNIX_GPG_KEY_ID and have gpg on PATH.

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Root) { $Root = (Get-Location).Path }
Set-Location $Root

$Dist = Join-Path $Root "dist\release-stage"
if (Test-Path $Dist) { Remove-Item $Dist -Recurse -Force }
New-Item -ItemType Directory -Path $Dist | Out-Null

Write-Host "==> cargo build --release -p rynixc"
cargo build -p rynixc --release
$exe = Join-Path $Root "target\release\rynixc.exe"
if (-not (Test-Path $exe)) { $exe = Join-Path $Root "target\release\rynixc" }
Copy-Item $exe (Join-Path $Dist (Split-Path $exe -Leaf)) -Force

foreach ($p in @("README.md", "LICENSE.md", "INSTALL.md", "AGENTS.md", "install.ps1", "INSTALL.sh")) {
  $src = Join-Path $Root $p
  if (Test-Path $src) { Copy-Item $src $Dist -Force }
}
Copy-Item (Join-Path $Root "std") (Join-Path $Dist "std") -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item (Join-Path $Root "examples") (Join-Path $Dist "examples") -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "==> SHA256SUMS"
Push-Location $Dist
Get-ChildItem -File | ForEach-Object {
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "{0}  {1}" -f $h, $_.Name
} | Set-Content -Encoding ascii SHA256SUMS.txt
Get-Content SHA256SUMS.txt
Pop-Location

$gpgId = $env:RYNIX_GPG_KEY_ID
if ($gpgId) {
  Write-Host "==> gpg --detach-sign SHA256SUMS.txt (key $gpgId)"
  Push-Location $Dist
  & gpg --batch --yes --local-user $gpgId --detach-sign --armor SHA256SUMS.txt
  Pop-Location
} else {
  Write-Host "Note: set RYNIX_GPG_KEY_ID to detach-sign SHA256SUMS.txt"
}

Write-Host "Staged: $Dist"
