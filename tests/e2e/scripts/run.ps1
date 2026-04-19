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

    $bin = Join-Path $repoRoot "target/release/mdview.exe"
    if (-not (Test-Path $bin)) {
        $bin = Join-Path $repoRoot "target/release/mdview"
    }

    $serverProc = $null
    try {
        Write-Host "==> Starting mdview server on 127.0.0.1:$Port"
        if (Test-Path $bin) {
            $everything = Join-Path $repoRoot "fixtures/everything.md"
            $serverProc = Start-Process -FilePath $bin -ArgumentList @("--serve-only", $everything) -PassThru -NoNewWindow
        } else {
            Write-Host "==> mdview binary missing, falling back to mdview-server demo_serve"
            $serverProc = Start-Process -FilePath "cargo" -ArgumentList @("run", "--release", "--example", "demo_serve") -WorkingDirectory (Join-Path $repoRoot "crates/mdview-server") -PassThru -NoNewWindow
        }

        Write-Host "==> Waiting for port $Port"
        $deadline = (Get-Date).AddSeconds(60)
        while ((Get-Date) -lt $deadline) {
            try {
                $client = [System.Net.Sockets.TcpClient]::new()
                $client.Connect("127.0.0.1", $Port)
                $client.Close()
                break
            } catch {
                Start-Sleep -Milliseconds 500
            }
        }

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

            npx playwright test --reporter=list
            $status = $LASTEXITCODE
        } finally {
            Pop-Location
        }
    } finally {
        if ($serverProc -and -not $serverProc.HasExited) {
            Write-Host "==> Stopping server (PID $($serverProc.Id))"
            Stop-Process -Id $serverProc.Id -Force -ErrorAction SilentlyContinue
        }
    }
} finally {
    Pop-Location
}

exit $status
