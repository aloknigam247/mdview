#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [int]$Port = 7681
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$e2eDir = Resolve-Path (Join-Path $here "..")
$repoRoot = Resolve-Path (Join-Path $e2eDir "../..")
$status = 1

Push-Location $repoRoot
try {
    Write-Host "==> Building workspace (release)"
    cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    Push-Location $e2eDir
    try {
        if (-not (Test-Path (Join-Path $e2eDir "node_modules"))) {
            Write-Host "==> Installing node deps"
            if (Get-Command bun -ErrorAction SilentlyContinue) {
                bun install
            } else {
                npm install
            }
        }

        npx playwright install chromium --with-deps 2>$null | Out-Null

        # Playwright's webServer block owns starting, port-waiting and tearing
        # down the mdview server. Do not start one here.
        $env:MDVIEW_E2E_PORT = "$Port"
        npx playwright test --reporter=list
        $status = $LASTEXITCODE
    } finally {
        Pop-Location
    }
} finally {
    Pop-Location
}

exit $status
