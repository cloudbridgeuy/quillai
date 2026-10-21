import { expect, type Locator } from "../app/node_modules/@playwright/test/index.js";

export async function expectEditorReady(editor: Locator) {
  await expect(editor).toHaveAttribute("contenteditable", "true");
  await expect(editor).toBeEditable();
}
