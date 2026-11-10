import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("caret line keeps its viewport position when the outline opens at 1100px", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Reflow series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Reflow book", position: 0 });
  const text = Array.from({ length: 100 }, (_, index) => `Reflow line ${index} with enough words to wrap once the editor narrows on open.\n`).join("");
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, { title: "Reflow chapter", position: 0, kind: "chapter", content: { ops: [{ insert: text }] } });
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expect(editor).toContainText("Reflow line 90");
  const line = editor.locator("p").nth(60);
  await line.scrollIntoViewIfNeeded();
  await line.click();
  const caretTop = () => page.evaluate(() => window.getSelection()!.getRangeAt(0).getBoundingClientRect().top);
  await expect.poll(caretTop).toBeGreaterThan(0);
  const before = await caretTop();
  const width = await editor.evaluate((element) => element.getBoundingClientRect().width);
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await expect(page.getByRole("button", { name: "Reflow chapter", exact: true })).toBeVisible();
  await expect.poll(() => editor.evaluate((element) => element.getBoundingClientRect().width)).toBeLessThan(width);
  await expect.poll(async () => Math.abs((await caretTop()) - before)).toBeLessThanOrEqual(2);
});
