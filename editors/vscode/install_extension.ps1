# Install Rynix VS Code extension (development copy)

$ErrorActionPreference = "Stop"
$CurrentDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $CurrentDir) { $CurrentDir = (Get-Location).Path }

Set-Location $CurrentDir
Write-Host "[1/2] Compiling TypeScript..." -ForegroundColor Yellow
if (-not (Test-Path "node_modules")) { npm install }
npm run compile

$version = (Get-Content package.json | ConvertFrom-Json).version
$Target = Join-Path $env:USERPROFILE ".vscode\extensions\rynix.rynix-lang-$version"

Write-Host "[2/2] Installing to $Target" -ForegroundColor Yellow
if (Test-Path $Target) { Remove-Item -Path $Target -Recurse -Force }
New-Item -ItemType Directory -Path $Target -Force | Out-Null

foreach ($f in @("package.json", "language-configuration.json", "README.md")) {
    Copy-Item -Path (Join-Path $CurrentDir $f) -Destination (Join-Path $Target $f) -Force
}
foreach ($d in @("dist", "syntaxes")) {
    Copy-Item -Path (Join-Path $CurrentDir $d) -Destination $Target -Recurse -Force
}

Write-Host "Installed Rynix VS Code extension v$version" -ForegroundColor Green
Write-Host "Restart VS Code/Cursor and open a .ryx file."
