#requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet("all", "drawio", "highlight", "math", "mermaid", "server", "webview")]
    [string]$Harness = "all",
    [ValidateRange(1, 2147483647)]
    [int]$StartupTimeoutMs = 300000,
    [ValidateRange(1, 2147483647)]
    [int]$TimeoutMs = 600000
)

$ErrorActionPreference = "Stop"
$cases = @(
    @{
        directory = "crates\mdview-ext-drawio"
        name = "drawio"
        port = 7685
        script = ".\tests\playwright_screenshot.mjs"
    },
    @{
        directory = "tests\e2e"
        name = "highlight"
        port = 7682
        script = ".\screenshot.mjs"
        server = @{
            arguments = @(
                "run", "--release", "--locked", "-p", "mdview", "--",
                "--serve-only", "--port", "7682", "fixtures\code.md"
            )
            pattern = "^mdview: serving on (?<url>http://127\.0\.0\.1:\d+)\r?$"
        }
    },
    @{
        directory = "crates\mdview-ext-math"
        name = "math"
        port = 7683
        script = ".\tests\playwright_screenshot.mjs"
        server = @{
            arguments = @("run", "--locked", "-p", "mdview-ext-math", "--example", "ext_math_serve")
            pattern = "^mdview-ext-math demo_serve listening on (?<url>http://127\.0\.0\.1:\d+)\r?$"
        }
    },
    @{
        directory = "crates\mdview-ext-mermaid\tests\e2e"
        name = "mermaid"
        port = 7684
        script = ".\playwright.spec.mjs"
        server = @{
            arguments = @("run", "--locked", "-p", "mdview-ext-mermaid", "--example", "ext_mermaid_serve")
            pattern = "^listening on (?<url>http://127\.0\.0\.1:\d+)\r?$"
        }
    },
    @{
        directory = "crates\mdview-server"
        name = "server"
        port = 0
        script = ".\tests\playwright_check.mjs"
        server = @{
            arguments = @("run", "--locked", "-p", "mdview-server", "--example", "server_serve")
            pattern = "^mdview-server listening on (?<url>http://127\.0\.0\.1:\d+)\r?$"
        }
    },
    @{
        directory = "apps\mdview\webview"
        name = "webview"
        port = 7681
        script = ".\playwright.mjs"
    }
)
$results = [System.Collections.Generic.List[object]]::new()
$repoRoot = Split-Path -Parent $PSScriptRoot
$artifactDir = Join-Path $repoRoot "artifacts\standalone\$([guid]::NewGuid())"
$selected = @($cases | Where-Object { $Harness -eq "all" -or $_.name -eq $Harness })

if (-not $IsWindows) { throw "The standalone fixture launcher requires Windows." }
if ($selected.Count -eq 0) { throw "No standalone harnesses were selected." }
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
Write-Host "Standalone diagnostics: $artifactDir"

function Test-PortOccupied([int]$Port) {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
    $listener.ExclusiveAddressUse = $true
    try {
        $listener.Start()
        return $false
    } catch [System.Net.Sockets.SocketException] {
        if ($_.Exception.SocketErrorCode -eq [System.Net.Sockets.SocketError]::AddressAlreadyInUse) {
            return $true
        }
        throw
    } finally {
        $listener.Stop()
    }
}

function Start-OwnedProcess {
    param(
        [string[]]$Arguments,
        [string]$Directory,
        [hashtable]$Environment,
        [string]$Executable,
        [string]$Name
    )

    $stderr = Join-Path $artifactDir "$Name.stderr.log"
    $stdout = Join-Path $artifactDir "$Name.stdout.log"
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Executable -ArgumentList $Arguments -Environment $Environment `
        -WorkingDirectory $Directory -RedirectStandardError $stderr -RedirectStandardOutput $stdout `
        -NoNewWindow -PassThru
    $result = [pscustomobject]@{
        elapsed = $null
        exit_code = $null
        name = $Name
        peak_working_set_bytes = 0L
        pid = $process.Id
        process = $process
        stderr = $stderr
        stdout = $stdout
        watch = $watch
    }
    $results.Add($result)
    Write-Host "$Name started as PID $($process.Id)"
    return $result
}

function Update-Peak($Result) {
    try {
        $Result.process.Refresh()
        if (-not $Result.process.HasExited) {
            $Result.peak_working_set_bytes = $Result.process.PeakWorkingSet64
        }
    } catch [System.InvalidOperationException] {
        if (-not $Result.process.HasExited) { throw }
    }
}

function Wait-OwnedProcess($Result) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while (-not $Result.process.WaitForExit(100)) {
        Update-Peak $Result
        if ($watch.ElapsedMilliseconds -ge $TimeoutMs) {
            throw "$($Result.name) timed out after $TimeoutMs ms; owned PID $($Result.pid)."
        }
    }
    $Result.exit_code = $Result.process.ExitCode
    if ($Result.exit_code -ne 0) {
        throw "$($Result.name) failed with exit code $($Result.exit_code)."
    }
}

function Wait-OwnedServer($Result, [string]$Pattern, [int]$ExpectedPort) {
    $lastError = "The owned server has not reported its listening address."
    $cancellation = [System.Threading.CancellationTokenSource]::new($StartupTimeoutMs)
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $handler.UseProxy = $false
    $client = [System.Net.Http.HttpClient]::new($handler)
    $client.Timeout = [System.Threading.Timeout]::InfiniteTimeSpan
    try {
        while (-not $cancellation.IsCancellationRequested) {
            if ($Result.process.HasExited) {
                throw "$($Result.name) exited before readiness with code $($Result.process.ExitCode)."
            }
            Update-Peak $Result
            $text = [string](Get-Content -LiteralPath $Result.stdout -Raw) + "`n" +
                [string](Get-Content -LiteralPath $Result.stderr -Raw)
            $match = [regex]::Match($text, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Multiline)
            if ($match.Success) {
                $url = [Uri]$match.Groups["url"].Value
                if ($ExpectedPort -ne 0 -and $url.Port -ne $ExpectedPort) {
                    throw "$($Result.name) reported an unexpected port: $($url.Port)."
                }
                try {
                    $response = $client.GetAsync(
                        $url, [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead, $cancellation.Token
                    ).GetAwaiter().GetResult()
                    try {
                        if (-not $response.IsSuccessStatusCode) {
                            throw "$($Result.name) readiness returned HTTP $([int]$response.StatusCode)."
                        }
                        return $url
                    } finally {
                        $response.Dispose()
                    }
                } catch [System.Net.Http.HttpRequestException] {
                    $lastError = $_.Exception.Message
                }
            }
            Start-Sleep -Milliseconds 100
        }
        throw "$($Result.name) readiness timed out after $StartupTimeoutMs ms. $lastError"
    } catch [System.OperationCanceledException] {
        throw "$($Result.name) readiness timed out after $StartupTimeoutMs ms. $lastError"
    } finally {
        $client.Dispose()
        $cancellation.Dispose()
    }
}

function Stop-OwnedProcess($Result) {
    if ($null -eq $Result) { return }
    try {
        if (-not $Result.process.HasExited) {
            Update-Peak $Result
            $Result.process.Kill($true)
        }
        if (-not $Result.process.WaitForExit(10000)) {
            throw "Owned PID $($Result.pid) did not exit during cleanup."
        }
        $Result.exit_code = $Result.process.ExitCode
    } finally {
        $Result.watch.Stop()
        $Result.elapsed = $Result.watch.Elapsed.ToString()
        $Result.process.Dispose()
    }
}

function Wait-PortReleased([int]$Port) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while (Test-PortOccupied $Port) {
        if ($watch.ElapsedMilliseconds -ge 10000) { throw "Port $Port was not released after cleanup." }
        Start-Sleep -Milliseconds 100
    }
}

try {
    $node = & node -p "process.execPath"
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $node -PathType Leaf)) {
        throw "Could not resolve the installed Node executable."
    }
    foreach ($case in $selected) {
        $portChecked = $false
        $server = $null
        $test = $null
        $environment = @{
            MDVIEW_ARTIFACTS_DIR = $artifactDir
            XDG_CONFIG_HOME = Join-Path $artifactDir "empty_config"
        }
        $port = $case.port
        try {
            if ($port -ne 0) {
                if (Test-PortOccupied $port) { throw "Port $port is occupied; no existing process will be reused." }
                $portChecked = $true
            }
            if ($case.ContainsKey("server")) {
                $cargo = (Get-Command cargo -CommandType Application -ErrorAction Stop |
                    Select-Object -First 1).Source
                $server = Start-OwnedProcess -Arguments $case.server.arguments -Directory $repoRoot `
                    -Environment $environment -Executable $cargo -Name "$($case.name)-server"
                $url = Wait-OwnedServer $server $case.server.pattern $port
                $port = $url.Port
                $portChecked = $true
                $environment.MDV_URL = $url.AbsoluteUri
                $environment.MDVIEW_URL = $url.AbsoluteUri
                $environment.MDV_SCREENSHOT = Join-Path $artifactDir "$($case.name).png"
            }
            $test = Start-OwnedProcess -Arguments @($case.script) -Directory (Join-Path $repoRoot $case.directory) `
                -Environment $environment -Executable $node -Name "$($case.name)-test"
            Wait-OwnedProcess $test
        } finally {
            try {
                Stop-OwnedProcess $test
            } finally {
                try {
                    Stop-OwnedProcess $server
                } finally {
                    foreach ($result in @($server, $test)) {
                        if ($null -ne $result) {
                            Write-Host "==> $($result.name) diagnostics"
                            Get-Content -LiteralPath $result.stdout -Tail 40
                            Get-Content -LiteralPath $result.stderr -Tail 40
                        }
                    }
                    if ($portChecked) { Wait-PortReleased $port }
                }
            }
        }
        Write-Host "$($case.name) passed; owned processes stopped and port $port released."
    }
} finally {
    $report = @($results | Select-Object elapsed, exit_code, name, peak_working_set_bytes, pid, stderr, stdout)
    ConvertTo-Json -InputObject $report -Depth 3 |
        Set-Content -LiteralPath (Join-Path $artifactDir "processes.json") -Encoding utf8
}
