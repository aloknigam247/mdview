# Browser tests

## Entrypoints

| File | Responsibility |
|------|----------------|
| [e2e/package.json](e2e/package.json) | Playwright script names |
| [e2e/playwright.config.ts](e2e/playwright.config.ts) | Main suite, app server, browser, and artifact configuration |
| [e2e/playwright_plotly.config.ts](e2e/playwright_plotly.config.ts) | Plotly example server and `unit-10.spec.ts` |
| [e2e/scripts/run.ps1](e2e/scripts/run.ps1) | Dependency/browser setup, release build, and both managed suites |
| [e2e/tests/](e2e/tests/) | Colorscheme, keymap, TOC, and zoom behavior |
| [e2e/tests/fixtures.spec.ts](e2e/tests/fixtures.spec.ts) | Fixture visibility, computed styles, and full-page captures |
| [run_standalone.ps1](run_standalone.ps1) | Standalone browser harness selection, owned processes, and diagnostics |

Rust unit/integration tests belong to their app or crate, not this directory.

## Commands

With dependencies and the matching browser installed, run from `tests\e2e`:

```ps1
bun run test:e2e --config playwright.config.ts
bun run test:e2e --config playwright_plotly.config.ts
```

For setup plus both suites, run from the workspace root:

```ps1
pwsh -NoProfile -File .\tests\e2e\scripts\run.ps1
```

For standalone cases, use `pwsh -NoProfile -File .\tests\run_standalone.ps1 -Harness <name>` from the workspace root.
Read that script's parameter declaration for accepted harness names; do not maintain a second list.
The complete CI selection and flags live in [.github/workflows/ci.yml](../.github/workflows/ci.yml).

## Ownership and assertions

- Acceptance requirements are in the [triage validation policy](../.github/triage.md#validation).
- Configuration isolation belongs to each harness. The main Playwright configuration supplies `XDG_CONFIG_HOME`;
  avoid tests that read or modify the user's config.
- Fixture source lives in [fixtures/](../fixtures/). Extend the nearest existing spec for changed behavior.
- Managed servers belong to Playwright's `webServer`: it starts, waits for, and stops them.
  Preserve `reuseExistingServer: false`; do not prelaunch a server or reuse an arbitrary listener.
  `MDVIEW_E2E_PORT` selects the main suite's port.
- Native-host behavior requires the app's Rust integration tests or native validation; `--serve-only` browser tests
  exercise the page, not the wry window, IPC host, or terminal pager.
