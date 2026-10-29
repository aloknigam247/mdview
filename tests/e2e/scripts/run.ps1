#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [int]$Port = 7681
)

$ErrorActionPreference = "Stop"
$status = 1
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$e2eDir = Resolve-Path (Join-Path $here "..")
$repoRoot = Resolve-Path (Join-Path $e2eDir "..\..")

Push-Location $repoRoot
try {
    Push-Location (Join-Path $repoRoot "apps\mdview\webview")
    try {
        bun install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "webview dependency install failed (exit $LASTEXITCODE)" }
        bun run build
        if ($LASTEXITCODE -ne 0) { throw "webview bundle build failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }

    Write-Host "==> Building workspace (release)"
    cargo build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    Push-Location $e2eDir
    try {
        bun install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "bun install failed (exit $LASTEXITCODE)" }
        node .\node_modules\@playwright\test\cli.js install chromium
        if ($LASTEXITCODE -ne 0) { throw "Chromium installation failed (exit $LASTEXITCODE)" }

        # Playwright's webServer block owns starting, port-waiting and tearing
        # down the mdview server. Do not start one here.
        $env:MDVIEW_E2E_PORT = "$Port"
        foreach ($config in @("playwright.config.ts", "playwright_plotly.config.ts")) {
            node .\node_modules\@playwright\test\cli.js test --config $config --workers=1 --forbid-only `
                --fail-on-flaky-tests --global-timeout=600000
            if ($LASTEXITCODE -ne 0) { throw "Playwright $config failed (exit $LASTEXITCODE)" }
        }
        $status = 0
    } finally {
        Pop-Location
    }
} finally {
    Pop-Location
}

exit $status
