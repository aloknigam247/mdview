# Creates a GitHub issue from a triage draft markdown file in one shot.
#
# The draft's first "# " heading becomes the issue title; the remainder is the body.
# Ensures each requested label exists (creates missing ones), then creates the
# issue assigned to the current user.
#
# Usage (run from anywhere inside the repo):
#   pwsh -NoProfile -ExecutionPolicy Bypass `
#     -File .github/skills/triage/scripts/new-issue.ps1 `
#     -Draft tmp/triage-issue.md -Label bug,tech-debt

param(
    [Parameter(Mandatory)][string]$Draft,    # path to draft md (first '# ' line = title)
    [Parameter(Mandatory)][string[]]$Label   # category labels, comma-separated
)

$ErrorActionPreference = "Stop"

# `pwsh -File` passes every argument as a literal string, so `-Label bug,tech-debt`
# arrives as the single element "bug,tech-debt" rather than a two-element array.
# Re-split so both that and native array invocation yield one label per element.
$Label = $Label -split "," | ForEach-Object { $_.Trim() } | Where-Object { $_ }
if (-not $Label) { throw "no labels supplied" }

# Operate from the repo root so the temp body file lands in the repo's tmp/.
$root = (git rev-parse --show-toplevel).Trim()
Set-Location $root

# --limit: `gh label list` returns only 30 labels by default.
$existing = gh label list --limit 200 --json name --jq ".[].name"
if ($LASTEXITCODE -ne 0) { throw "could not list existing labels" }
foreach ($l in $Label) {
    if ($existing -notcontains $l) {
        gh label create "$l" --description "$l" --force | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "could not create label '$l'" }
    }
}

$lines = Get-Content $Draft
$title = ($lines | Where-Object { $_ -match '^# ' } | Select-Object -First 1) -replace '^# ', ''
$idx = ($lines | Select-String -Pattern '^# ' | Select-Object -First 1).LineNumber

New-Item -ItemType Directory -Force -Path "tmp" | Out-Null
$body = "tmp/triage-body-$([guid]::NewGuid().ToString()).md"
($lines[$idx..($lines.Count - 1)]) | Set-Content -Encoding utf8 $body

$labelArgs = @()
foreach ($l in $Label) { $labelArgs += @("--label", $l) }

$url = gh issue create --title "$title" --body-file "$body" @labelArgs --assignee "@me"
$createExit = $LASTEXITCODE
Remove-Item $body -ErrorAction SilentlyContinue
if ($createExit -ne 0 -or -not $url) { throw "gh issue create failed (exit $createExit)" }

Write-Output "TITLE: $title"
Write-Output "URL: $url"
