import { defineConfig } from "../app/node_modules/@playwright/test/index.js";

export default defineConfig({
  testDir: ".",
  testMatch: ["full-prompt.spec.ts", "server-errors.spec.ts", "ai-shortcuts.spec.ts", "design-cleanup.spec.ts", "design-tokens.spec.ts", "type-tokens.spec.ts", "inspector-cost.spec.ts", "inspector-selection.spec.ts", "inspector-fields.spec.ts", "editor-buttons.spec.ts", "rail-buttons.spec.ts", "alerts-toggles.spec.ts", "ai-thread.spec.ts", "continue-inline.spec.ts", "continue-no-repeat.spec.ts", "continue-reasoning.spec.ts", "chapter-sync.spec.ts", "chapter-emoji-sync.spec.ts", "author-guidance.spec.ts", "continue-stream.spec.ts", "desktop-edge-cases.spec.ts", "continue-window.spec.ts", "lore.spec.ts", "lore-modal.spec.ts", "save-recovery.spec.ts", "model-picker.spec.ts", "offline-stale-base.spec.ts", "reload-in-flight.spec.ts", "emoji-cursor-remote.spec.ts", "stream-remote-edit.spec.ts", "real-route-stream.spec.ts", "continue-tie-order.spec.ts", "continue-empty.spec.ts", "continue-poll.spec.ts", "continue-unsaved.spec.ts", "tree-load-shift.spec.ts", "tree-active-scroll.spec.ts", "edit-replacement.spec.ts", "edit-selection-highlight.spec.ts", "edit-tint.spec.ts", "edit-note.spec.ts", "edit-lore-popover.spec.ts", "ui-components.spec.ts", "settings-popover.spec.ts", "persistent-dev-database.spec.ts", "manuscript-create.spec.ts", "create-work.spec.ts", "tab-title.spec.ts", "story-tree.spec.ts", "lore-scope.spec.ts", "e2e-styles.spec.ts", "create-form-fit.spec.ts", "piece-move-lore.spec.ts", "landing.spec.ts", "screenplay-scroll.spec.ts", "screenplay-cross-scene.spec.ts", "screenplay-ai.spec.ts", "screenplay-notice.spec.ts", "screenplay-new-scene.spec.ts", "screenplay-format.spec.ts", "screenplay-fdx.spec.ts"],
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
