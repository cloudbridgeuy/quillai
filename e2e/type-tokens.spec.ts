import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openEditor(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Type tokens", position: 0 } })).json();
  const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
  const content = `${Array.from({ length: 50 }, (_, index) => `${index + 1}. ${"A long chapter line. ".repeat(8)}`).join("\n")}\n`;
  const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: content }] },
  } })).json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
}

for (const width of [1100, 1440]) {
  test.describe(`${width}px`, () => {
    test.beforeEach(async ({ page, request }) => {
      await openEditor(page, request, width);
    });

    test("field labels use 12px", async ({ page }) => {
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      const settings = page.getByRole("dialog", { name: "Settings", exact: true });
      const label = settings.getByText("Window Size", { exact: true });
      await expect(settings.getByRole("spinbutton", { name: "Window Size", exact: true })).toBeVisible();
      await expect(label).toBeVisible();
      await expect(label).toHaveCSS("font-size", "12px");
    });

    test("small button uses 12px", async ({ page }) => {
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      const button = page.getByRole("button", { name: "Save author guidance", exact: true });
      await expect(button).toBeVisible();
      await expect(button).toHaveCSS("font-size", "12px");
    });

    test("rail glyph uses 16px", async ({ page }) => {
      const glyph = page.getByRole("button", { name: "Manuscript", exact: true }).locator("span");
      await expect(glyph).toBeVisible();
      await expect(glyph).toHaveText("☰");
      await expect(glyph).toHaveCSS("font-size", "16px");
    });

    test("top bar title uses 16px", async ({ page }) => {
      const title = page.getByRole("heading", { name: "QuillAI", exact: true });
      await expect(title).toBeVisible();
      await expect(title).toHaveCSS("font-size", "16px");
    });

    test("sheet text stays 21px", async ({ page }) => {
      const text = page.locator(".ql-editor p").first();
      await expect(text).toBeVisible();
      await expect(text).toHaveCSS("font-size", "21px");
    });

    test("Continue window flag stays 11px", async ({ page }) => {
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
      await expect(flag).toBeVisible();
      await expect(flag).toHaveCSS("font-size", "11px");
    });
  });
}

for (const width of [1100, 1440]) {
  test.describe(`additional type tokens at ${width}px`, () => {
    test.beforeEach(async ({ page, request }) => {
      await openEditor(page, request, width);
    });

    test("editor note uses 13px", async ({ page }) => {
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      const note = page.getByText("Saved on the server for all requests. Empty text removes guidance. Limit: 65,536 UTF-8 bytes. System rules are read-only.", { exact: true });
      await expect(note).toBeVisible();
      await expect(note).toHaveCSS("font-size", "13px");
    });

    test("preview section token count uses 13px", async ({ page }) => {
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      const count = page.getByRole("heading", { name: "System", exact: true }).locator("..").locator("span");
      await expect(count).toBeVisible();
      await expect(count).toHaveCSS("font-size", "13px");
    });

    test("chapter title uses 0.2em letter spacing", async ({ page }) => {
      const title = page.getByRole("heading", { name: "Chapter", exact: true });
      await expect(title).toBeVisible();
      await expect(title).toHaveCSS("font-size", "13px");
      await expect(title).toHaveCSS("letter-spacing", "2.6px");
    });
  });
}
