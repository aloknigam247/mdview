import { defineConfig, devices } from "@playwright/test";
import { resolve } from "node:path";

const baseURL = "http://127.0.0.1:7681";
const repoRoot = resolve(__dirname, "..", "..", "..", "..");

export default defineConfig({
  testDir: ".",
  testMatch: "screenshot.spec.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: {
    actionTimeout: 15_000,
    baseURL,
    navigationTimeout: 30_000,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  outputDir: "artifacts/test-results",
  webServer: {
    command: "cargo run --locked -p mdview-render-html --example render_html_serve",
    cwd: repoRoot,
    reuseExistingServer: false,
    stderr: "pipe",
    stdout: "pipe",
    timeout: 300_000,
    url: baseURL,
  },
});
