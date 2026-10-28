import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("Continue does not repeat the passage before the caret", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identity = `GUZ870-${crypto.randomUUID()}`;
  const tail = "The lantern flickered at the window.";
  const continuation = "Beyond it, the rain began.";
  const opening = `Mara waited ${identity}. ${tail}`;
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Continue series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Continue book", position: 0 });
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await post(chapters, { title: "Continue chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `${opening}\n` }] } });
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  const status = page.getByRole("status", { name: "Chapter save status", exact: true });
  await expectEditorReady(editor);
  await expect(status).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByText("Advanced AI controls", { exact: true }).click();
  await page.getByRole("combobox", { name: "Continue model", exact: true }).selectOption("e2e-repeat");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await editor.evaluate(element => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing paragraph text");
    (element as HTMLElement).focus();
    const range = document.createRange();
    range.setStart(text, text.textContent!.length);
    range.collapse(true);
    const selection = window.getSelection()!;
    selection.removeAllRanges();
    selection.addRange(range);
  });
  expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-repeat" } })).status()).toBe(200);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(editor).toContainText(continuation);
  await expect(status).toHaveText("Saved");
  const uiText = await editor.innerText();
  expect.soft(uiText.split(tail)).toHaveLength(2);
  expect.soft(uiText.indexOf(continuation)).toBeGreaterThan(uiText.indexOf(tail));
  await expect.poll(async () => {
    const response = await request.get(`${chapters}/${chapter.id}`);
    expect(response.status()).toBe(200);
    return JSON.stringify((await response.json()).content);
  }).toContain(continuation);
  const response = await request.get(`${chapters}/${chapter.id}`);
  expect(response.status()).toBe(200);
  const saved = JSON.stringify((await response.json()).content);
  expect(saved.split(tail)).toHaveLength(2);
  expect(saved.indexOf(continuation)).toBeGreaterThan(saved.indexOf(tail));
});
