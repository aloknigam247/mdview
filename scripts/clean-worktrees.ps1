#!/usr/bin/env pwsh
# Remove stale Claude agent worktrees and their branches.
#
# Background agents launched with `isolation: "worktree"` leave behind
# `.claude/worktrees/agent-<id>` directories and `worktree-agent-<id>` branches.
# Run this at the end of a session to clean them all up in one shot.
#
# Usage:
#   pwsh scripts/clean-worktrees.ps1            # dry run, lists what would happen
#   pwsh scripts/clean-worktrees.ps1 -Apply     # actually delete

[CmdletBinding()]
param(
    [switch]$Apply
)

$ErrorActionPreference = "Stop"

$worktrees = @()
foreach ($line in (git worktree list --porcelain)) {
    if ($line -match "^worktree (.+)") {
        $path = $Matches[1]
        if ($path -match "[\\/]\.claude[\\/]worktrees[\\/]agent-") {
            $worktrees += $path
        }
    }
}

$branches = @()
foreach ($line in (git branch --list "worktree-agent-*" --format="%(refname:short)")) {
    if ($line) { $branches += $line.Trim() }
}

if ($worktrees.Count -eq 0 -and $branches.Count -eq 0) {
    Write-Host "Nothing to clean. No agent worktrees or branches found." -ForegroundColor Green
    exit 0
}

Write-Host ("Found {0} worktree(s) and {1} branch(es):" -f $worktrees.Count, $branches.Count)
foreach ($w in $worktrees) { Write-Host "  worktree: $w" }
foreach ($b in $branches)  { Write-Host "  branch:   $b" }

if (-not $Apply) {
    Write-Host ""
    Write-Host "Dry run. Re-run with -Apply to actually delete." -ForegroundColor Yellow
    exit 0
}

foreach ($w in $worktrees) {
    Write-Host "Removing worktree $w ..."
    git worktree remove -f -f $w 2>&1 | Out-Host
}

foreach ($b in $branches) {
    Write-Host "Deleting branch $b ..."
    git branch -D $b 2>&1 | Out-Host
}

Write-Host ""
Write-Host "Done." -ForegroundColor Green
