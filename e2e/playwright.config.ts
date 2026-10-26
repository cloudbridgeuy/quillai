import { defineConfig } from "../app/node_modules/@playwright/test/index.js";

export default defineConfig({
  testDir: ".",
  testMatch: ["chapter-sync.spec.ts", "chapter-emoji-sync.spec.ts", "author-guidance.spec.ts", "continue-stream.spec.ts", "desktop-edge-cases.spec.ts", "continue-window.spec.ts", "lore.spec.ts", "save-recovery.spec.ts", "model-picker.spec.ts", "offline-stale-base.spec.ts", "reload-in-flight.spec.ts", "emoji-cursor-remote.spec.ts", "stream-remote-edit.spec.ts", "real-route-stream.spec.ts", "continue-tie-order.spec.ts", "continue-poll.spec.ts", "tree-load-shift.spec.ts", "tree-active-scroll.spec.ts", "edit-replacement.spec.ts", "ui-components.spec.ts", "persistent-dev-database.spec.ts", "manuscript-create.spec.ts", "create-work.spec.ts", "tab-title.spec.ts", "story-tree.spec.ts", "lore-scope.spec.ts"],
  outputDir: "../target/xtask-e2e/results",
  reporter: process.env.QUILLAI_E2E_SUMMARY ? [["list"], ["./summary-reporter.ts"]] : "list",
  workers: 1,
  retries: 0,
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
  timeout: 60_000,
  expect: { timeout: 15_000 },
  use: {
    baseURL: process.env.QUILLAI_E2E_APP_URL ?? "http://127.0.0.1:3001",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
});
