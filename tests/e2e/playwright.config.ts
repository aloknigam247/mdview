import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 60_000,
  use: {
    baseURL: "http://127.0.0.1:7686",
  },
  reporter: [["list"]],
});
