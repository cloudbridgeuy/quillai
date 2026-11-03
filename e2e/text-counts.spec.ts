import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("document and selection counts follow editing", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Counts", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Book", position: 0 });
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "\n" }] },
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  const counts = page.getByLabel("Document counts");
  await expectEditorReady(editor);
  await expect(counts).toHaveText("0 words · 0 characters · 1 line");
  await editor.click();
  await page.keyboard.insertText("hello world 🪐");
  await expect(counts).toHaveText("2 words · 13 characters · 1 line");
  await editor.evaluate((element) => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing editor text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 5);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await expect(counts).toHaveText("1 of 2 words · 13 characters · 1 line");
});
