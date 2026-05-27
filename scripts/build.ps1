#requires -Version 7
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repo = Split-Path -Parent $PSScriptRoot
$target = Join-Path $repo "target\release"
$dotfiles = "D:\dotfiles\mdview"

Write-Host "`u{2192} cargo build --workspace --release" -ForegroundColor Cyan
Push-Location $repo
try {
    cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
} finally { Pop-Location }

Write-Host "`u{2192} sidecar bun build" -ForegroundColor Cyan
$sidecar = Join-Path $repo "sidecar"
if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
    throw "bun not found on PATH (required for sidecar build)"
}
$sidecarOut = Join-Path $target "mdview-sidecar.exe"
Push-Location $sidecar
try {
    if (-not (Test-Path (Join-Path $sidecar "node_modules"))) {
        Write-Host "  bun install (first run)" -ForegroundColor DarkGray
        bun install
        if ($LASTEXITCODE -ne 0) { throw "bun install failed (exit $LASTEXITCODE)" }
    }
    bun build --compile .\src\index.ts --outfile $sidecarOut
    if ($LASTEXITCODE -ne 0) { throw "sidecar build failed (exit $LASTEXITCODE)" }
} finally { Pop-Location }

Write-Host "`u{2192} install to $dotfiles" -ForegroundColor Cyan
if (-not (Test-Path $dotfiles)) { New-Item -ItemType Directory -Path $dotfiles | Out-Null }
Copy-Item (Join-Path $target "mdview.exe")         (Join-Path $dotfiles "mdview.exe")         -Force
Copy-Item (Join-Path $target "mdview-sidecar.exe") (Join-Path $dotfiles "mdview-sidecar.exe") -Force

Write-Host "`u{2713} done" -ForegroundColor Green
