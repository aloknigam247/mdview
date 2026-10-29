import { defineConfig, devices } from "@playwright/test";
import { resolve } from "node:path";

const address = "127.0.0.1:7686";
const baseURL = `http://${address}`;
const repoRoot = resolve(__dirname, "..", "..");

export default defineConfig({
  forbidOnly: !!process.env.CI,
  fullyParallel: false,
  outputDir: "artifacts/plotly/test-results",
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  reporter: [["list"], ["html", { open: "never", outputFolder: "artifacts/plotly/report" }]],
  retries: 0,
  testDir: ".",
  testMatch: "unit-10.spec.ts",
  timeout: 60_000,
  use: {
    actionTimeout: 15_000,
    baseURL,
    navigationTimeout: 30_000,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure",
  },
  webServer: {
    command: "cargo run --locked -p mdview-ext-plotly --example ext_plotly_serve",
    cwd: repoRoot,
    env: { MDV_PLOTLY_ADDR: address },
    reuseExistingServer: false,
    stderr: "pipe",
    stdout: "pipe",
    timeout: 300_000,
    url: baseURL,
  },
  workers: 1,
});
