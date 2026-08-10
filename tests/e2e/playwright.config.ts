import { defineConfig, devices } from "@playwright/test";
import { resolve } from "node:path";

const port = Number(process.env.MDVIEW_E2E_PORT ?? 7681);
const baseURL = `http://127.0.0.1:${port}`;
const repoRoot = resolve(__dirname, "..", "..");

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 1,
  workers: 1,
  reporter: [["list"], ["html", { open: "never", outputFolder: "playwright-report" }]],
  outputDir: "artifacts/test-results",
  use: {
    baseURL,
    screenshot: "only-on-failure",
    trace: "on-first-retry",
    video: "retain-on-failure",
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
  },
  // `cargo run` rather than a prebuilt target/release path: `bun run test:e2e`
  // is a bare `playwright test`, so without this a stale binary (or a clean
  // checkout with none at all) would silently be what gets tested.
  webServer: {
    command: `cargo run --release -p mdview -- --serve-only --port ${port} fixtures/everything.md`,
    cwd: repoRoot,
    url: baseURL,
    // Never reuse: a stray process on this port would both taint results and
    // be deliberately left running, which is the leak the suite must not have.
    reuseExistingServer: false,
    timeout: 300_000,
    stdout: "pipe",
    stderr: "pipe",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
