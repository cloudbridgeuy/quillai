import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function edit(page: Page, request: APIRequestContext, width: number, tint: boolean) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  const providerUrl = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!apiUrl || !providerUrl) throw new Error("Run with cargo xtask e2e and its default catalog");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Edit tint", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Edit tint", position: 0 } })).json();
  const response = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Edit tint", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\nLater paragraph\n" }] },
  } });
  expect(response.status()).toBe(201);
  const chapter = await response.json();
  await page.setViewportSize({ width: Math.max(width, 1100), height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(editor.locator("p")).toHaveText(["Opening line", "Later paragraph"]);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  if (!tint) {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("checkbox", { name: "Tint AI text", exact: true }).uncheck();
    await page.getByRole("button", { name: "Settings", exact: true }).click();
  }
  const identity = `GUZ985-edit-${crypto.randomUUID()}`;
  const before = (await (await request.get(`${providerUrl}/control`)).json()).length as number;
  expect((await request.post(`${providerUrl}/control/arm`, { data: { identity, model: "e2e-edit" } })).status()).toBe(200);
  await editor.click();
  await editor.evaluate(element => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 7);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  await page.getByRole("textbox", { name: "Edit instruction", exact: true }).fill(identity);
  const generation = page.waitForResponse(response => response.url().endsWith("/ai/generate") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  const controlUrl = `${providerUrl}/control/${before}`;
  expect((await request.get(`${controlUrl}/request`)).status()).toBe(200);
  expect((await request.post(`${controlUrl}/fragment`, { data: { text: "<replacement>Revised</replacement>" } })).status()).toBe(200);
  expect((await generation).status()).toBe(200);
  await expect(editor.locator("p").first()).toHaveText("Revised line");
  if (width < 1100) await page.setViewportSize({ width, height: 1000 });
  await editor.evaluate(element => {
    const state = window as typeof window & { editTintSnapshot?: unknown };
    const observer = new MutationObserver(() => {
      if (element.getAttribute("contenteditable") !== "true") return;
      state.editTintSnapshot = {
        tint: element.getAttribute("data-ai-tint"),
        stream: Array.from(CSS.highlights.get("quillai-stream") ?? []).map(range => range.toString()),
        selection: Array.from(CSS.highlights.get("quillai-edit-selection") ?? []).map(range => range.toString()),
      };
      observer.disconnect();
    });
    observer.observe(element, { attributes: true, attributeFilter: ["contenteditable"] });
  });
  expect((await request.post(`${controlUrl}/finish`)).status()).toBe(200);
  await expect.poll(() => page.evaluate(() => (window as typeof window & { editTintSnapshot?: unknown }).editTintSnapshot)).toBeDefined();
  const snapshot = await page.evaluate(() => (window as typeof window & { editTintSnapshot?: unknown }).editTintSnapshot);
  return { editor, snapshot };
}

for (const width of [1100, 1440]) {
  test(`Applied edit fades and typing clears its tint at ${width}px`, async ({ page, request }) => {
    const { editor, snapshot } = await edit(page, request, width, true);
    expect(snapshot).toEqual({ tint: "fading", stream: ["Revised"], selection: [] });
    await editor.click();
    await page.keyboard.type("x");
    await expect(editor).not.toHaveAttribute("data-ai-tint");
    expect(await page.evaluate(() => Array.from(CSS.highlights.get("quillai-stream") ?? []))).toEqual([]);
  });

  test(`Applied edit has no tint when disabled at ${width}px`, async ({ page, request }) => {
    const { snapshot } = await edit(page, request, width, false);
    expect(snapshot).toEqual({ tint: null, stream: [], selection: [] });
  });
}

test("Applied edit has no tint below 1100px", async ({ page, request }) => {
  const { snapshot } = await edit(page, request, 1099, true);
  expect(snapshot).toEqual({ tint: null, stream: [], selection: [] });
});
