import { defineConfig } from "../app/node_modules/@playwright/test/index.js";

export default defineConfig({
  testDir: ".",
  testMatch: ["chapter-sync.spec.ts", "chapter-emoji-sync.spec.ts", "author-guidance.spec.ts", "continue-stream.spec.ts", "desktop-edge-cases.spec.ts", "continue-window.spec.ts", "lore.spec.ts", "save-recovery.spec.ts"],
  outputDir: "../target/xtask-e2e/results",
  reporter: "list",
  workers: 1,
  retries: 0,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  use: {
    browserName: "chromium",
    baseURL: process.env.QUILLAI_E2E_APP_URL ?? "http://127.0.0.1:3001",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
});
