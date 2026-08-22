# Install rynixc from this repository (Windows PowerShell).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "==> Building and installing rynixc"
cargo install --path crates/rynixc --force
$rynixc = (Get-Command rynixc -ErrorAction SilentlyContinue).Source
if (-not $rynixc) {
  Write-Host "rynixc installed, but not on PATH yet. Add Cargo's bin dir:"
  Write-Host "  $env:USERPROFILE\.cargo\bin"
  exit 0
}
Write-Host "Installed: $rynixc"
& rynixc --version
Write-Host ""
Write-Host "Try:  rynixc run examples/01_hello.ryx"
Write-Host "      rynixc arch check"
Write-Host "      .\editors\vscode\install_extension.ps1"
Write-Host "Docs: INSTALL.md  README.md"
