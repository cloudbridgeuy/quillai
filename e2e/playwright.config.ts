import { defineConfig } from "../app/node_modules/@playwright/test/index.mjs";

export default defineConfig({
  testDir: ".",
  testMatch: "chapter-sync.spec.ts",
  outputDir: "../target/xtask-e2e/results",
  reporter: "list",
  workers: 1,
  retries: 0,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  use: {
    browserName: "chromium",
    baseURL: "http://127.0.0.1:3001",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
});
