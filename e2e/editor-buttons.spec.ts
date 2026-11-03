import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test.use({ viewport: { width: 1440, height: 1000 } });

function apiUrl() {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  return api;
}

async function post(request: APIRequestContext, url: string, data: object) {
  const response = await request.post(url, { data });
  expect(response.status()).toBe(201);
  return response.json();
}

async function chapter(page: Page, request: APIRequestContext) {
  const api = apiUrl();
  const series = await post(request, `${api}/series`, { title: "Button series", position: 0 });
  const book = await post(request, `${api}/series/${series.id}/books`, { title: "Button book", position: 0 });
  const content = `${Array.from({ length: 50 }, (_, index) => `${index + 1}. ${"A long chapter line. ".repeat(8)}`).join("\n")}\n`;
  const entry = await post(request, `${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Button chapter", position: 0, kind: "chapter", content: { ops: [{ insert: content }] },
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(entry.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
}

async function expectShape(control: Locator, name: string | RegExp) {
  await expect(control).toBeVisible();
  await expect(control).toHaveAccessibleName(name);
  const shape = await control.evaluate(element => ({ radius: parseFloat(getComputedStyle(element).borderTopLeftRadius), height: element.getBoundingClientRect().height }));
  expect(shape.height, `${name} height`).toBeGreaterThan(0);
  expect.soft(shape.radius, `${name} radius`).toBeLessThan(shape.height / 2);
}

test("create chapter", async ({ page }) => {
  const api = apiUrl();
  await page.route(`${api}/series`, route => route.fulfill({ json: [] }));
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
  await expectShape(page.getByRole("button", { name: "Create the first chapter", exact: true }), "Create the first chapter");
});

test("editor and lore controls", async ({ page, request }) => {
  let failLore = true;
  await page.route(`${apiUrl()}/universes/*/lore`, route => failLore
    ? route.fulfill({ status: 500, json: { error: "Lore unavailable", code: "ERROR" } }) : route.continue());
  await chapter(page, request);
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const lore = page.getByRole("region", { name: "Series lore", exact: true });
  await expectShape(page.getByRole("link", { name: "Export local chapter", exact: true }), "Export local chapter");
  const retry = lore.getByRole("button", { name: "Retry lore", exact: true });
  await expectShape(retry, "Retry lore");
  failLore = false;
  await retry.click();
  const newLore = lore.getByRole("button", { name: "New lore", exact: true });
  await expectShape(newLore, "New lore");
  await newLore.click();
  const dialog = page.getByRole("dialog", { name: "Create lore", exact: true });
  await dialog.getByLabel("Name", { exact: true }).fill("Mira");
  await dialog.getByRole("button", { name: "Save lore", exact: true }).click();
  await expectShape(lore.getByRole("button", { name: "Mira · character", exact: true }), "Mira · character");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expectShape(page.getByRole("button", { name: "Save author guidance", exact: true }), "Save author guidance");
  for (const name of ["Add user turn", "Add assistant turn"]) {
    await expectShape(page.getByRole("button", { name, exact: true }), name);
  }
  await page.getByRole("button", { name: "Add user turn", exact: true }).click();
  await expectShape(page.getByRole("button", { name: "Reset History", exact: true }), "Reset History");
});

test("screenplay download links", async ({ page, request }) => {
  const api = apiUrl();
  const universe = await post(request, `${api}/universes`, { title: "Button script", position: 0 });
  const group = await post(request, `${api}/universes/${universe.id}/pieces`, { title: "Script", position: 0, label: "screenplay", shape: "group" });
  await post(request, `${api}/pieces/${group.id}/children`, { title: "Scene", position: 0, label: "scene" });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  for (const name of ["Download Fountain", "Download FDX"]) {
    await expectShape(page.getByRole("link", { name, exact: true }), name);
  }
});

test("continue window flag", async ({ page, request }) => {
  await chapter(page, request);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("checkbox", { name: "Whole document", exact: true }).check();
  await page.keyboard.press("Escape");
  await page.locator(".ql-editor").evaluate(editor => {
    const node = editor.querySelectorAll("p")[30]!.firstChild!;
    (editor as HTMLElement).focus({ preventScroll: true });
    const range = document.createRange();
    range.setStart(node, 10);
    range.collapse(true);
    window.getSelection()!.removeAllRanges();
    window.getSelection()!.addRange(range);
  });
  await page.locator(".ql-editor p").nth(30).scrollIntoViewIfNeeded();
  const flag = page.getByRole("button", { name: /Continue window starts ↑/ });
  await expectShape(flag, /Continue window starts ↑ (just above|\d+ pages up)/);
});
