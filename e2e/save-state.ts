import { expect, type Page } from "../app/node_modules/@playwright/test/index.js";

export async function expectSaved(page: Page) {
  await expect(page.locator(".editor-shell")).toHaveAttribute("data-save-state", "saved");
}
