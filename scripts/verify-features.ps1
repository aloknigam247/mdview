#!/usr/bin/env pwsh
# Verify that named features are actually present in the source tree.
#
# After background agents complete (especially worktree-isolated ones), use
# this to confirm their work landed in main rather than getting lost in a
# leaky auto-merge. Each row is (label, pattern, file). Exit code is 0 if all
# rows have >=1 match, 1 otherwise.
#
# Usage:
#   pwsh scripts/verify-features.ps1                   # default checks
#   pwsh scripts/verify-features.ps1 -Verbose          # show full results
#
# Add new sentinels here as the project grows.

[CmdletBinding()]
param(
    [string]$Root = (Resolve-Path "$PSScriptRoot/..").Path
)

$checks = @(
    # label                          # pattern                                  # relative path
    @("Ctrl+Q IPC",                  "MdvUserEvent::Quit",                       "apps/mdview/src/pipeline.rs"),
    @("Window icon",                 "with_window_icon",                         "apps/mdview/src/pipeline.rs"),
    @("Icon loader",                 "load_icon",                                "apps/mdview/src/pipeline.rs"),
    @("File-name title",             "format_window_title",                      "apps/mdview/src/pipeline.rs"),
    @("DWM titlebar",                "apply_dwm_theme",                          "apps/mdview/src/pipeline.rs"),
    @("Image URL rewrite",           "rewrite_image_urls",                       "apps/mdview/src/render.rs"),
    @("Custom protocol base",        "MDVIEW_PROTOCOL_BASE",                     "apps/mdview/src/render.rs"),
    @("Frontmatter pre_render",      "pre_render_html",                          "apps/mdview/src/render.rs"),
    @("Frontmatter pre_render term", "pre_render_terminal",                      "apps/mdview/src/render_terminal.rs"),
    @("FrontmatterExt registered",   "FrontmatterExt",                           "apps/mdview/src/builtins.rs"),
    @("TOC element",                 "mdv-toc-nav",                              "apps/mdview/src/render.rs"),
    @("TOC toggle JS",               "__mdvToggleToc",                           "apps/mdview/src/render.rs"),
    @("Codemap pointer-capture",     "setPointerCapture",                        "apps/mdview/src/render.rs"),
    @("Codemap drag class",          "mdv-codemap-dragging",                     "apps/mdview/src/render.rs"),
    @("Lazy features",               "fn detect",                                "apps/mdview/src/render.rs"),
    @("Ctrl+wheel zoom",             "setupZoom",                                "apps/mdview/src/render.rs"),
    @("Bionic transform",            "setupBionic",                              "apps/mdview/src/render.rs"),
    @("Context menu",                "mdv-context-menu",                         "apps/mdview/src/render.rs"),
    @("Config error banner",         "mdv-config-banner",                        "apps/mdview/src/render.rs"),
    @("Build icon pipeline",         "build_icons",                              "apps/mdview/build.rs"),
    @("winresource embed",           "winresource",                              "apps/mdview/build.rs")
)

$ok = 0; $fail = 0
foreach ($row in $checks) {
    $label, $pattern, $rel = $row
    $full = Join-Path $Root $rel
    if (-not (Test-Path $full)) {
        Write-Host ("  MISSING  {0,-30} (file not found: {1})" -f $label, $rel) -ForegroundColor Red
        $fail++; continue
    }
    $count = (Select-String -Path $full -Pattern $pattern -SimpleMatch -AllMatches | Measure-Object).Count
    if ($count -gt 0) {
        Write-Host ("  OK ({0,3}) {1,-30} {2}" -f $count, $label, $rel) -ForegroundColor Green
        $ok++
    } else {
        Write-Host ("  MISSING  {0,-30} {1}" -f $label, $rel) -ForegroundColor Red
        $fail++
    }
}

Write-Host ""
Write-Host ("Result: {0} present, {1} missing" -f $ok, $fail)
if ($fail -gt 0) { exit 1 } else { exit 0 }
