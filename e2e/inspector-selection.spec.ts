import { expect, test, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

async function selectText(editor: Locator) {
  await editor.click();
  await editor.evaluate(element => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 2);
    range.setEnd(text, 15);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
}

async function openInspector(page: Page) {
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  return page.locator("#context-drawer");
}

for (const openFirst of [false, true]) {
  test(`Continue inspector shows caret ${openFirst ? "after" : "before"} selecting text`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
    const identity = `GUZ868-1-${crypto.randomUUID()}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: "Inspector selection", position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: "Book", position: 0 });
    const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
      title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `A quiet harbour. ${identity}\n` }] },
    });
    await page.setViewportSize({ width: 1100, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await expectSaved(page);
    const inspector = openFirst ? await openInspector(page) : page.locator("#context-drawer");
    if (openFirst) await expect(inspector.getByRole("textbox", { name: "Selection prompt section" })).toHaveValue("<selection>\nnone\n</selection>\n");
    await selectText(editor);
    await expect(page.getByRole("button", { name: "Edit selection", exact: true })).toBeEnabled();
    if (!openFirst) await openInspector(page);
    const selection = inspector.getByRole("textbox", { name: "Selection prompt section" });
    await expect(selection).toHaveValue("Caret after “quiet harbour”: Continue writes here.");
    await expect(selection).toHaveAttribute("readonly", "");
    const section = selection.locator("xpath=ancestor::section[1]");
    const tokenLabel = await section.locator(".editor-preview-section-header span").innerText();
    await inspector.getByText("Advanced AI controls", { exact: true }).click();
    await inspector.getByRole("combobox", { name: "Request to inspect" }).selectOption("edit");
    await expect(selection).toHaveValue("<selection>\nindex=2, length=13, text=quiet harbour\n</selection>\n");
    await expect(selection).toHaveAttribute("readonly", "");
    await inspector.getByRole("combobox", { name: "Request to inspect" }).selectOption("generate");
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
    const sent = page.waitForRequest(req => req.url() === `${api}/ai/generate` && req.method() === "POST");
    await page.getByRole("button", { name: /^Continue writing/ }).click();
    const payload = (await sent).postDataJSON();
    expect(payload.sectionOverrides?.selection).toBeUndefined();
    expect(payload.selection.text).toBe("");
    expect(payload.selection.range).toEqual({ index: 15, length: 0 });
    const wrapper = `<selection>\nindex=${payload.selection.range.index}, length=${payload.selection.range.length}, text=\n</selection>\n`;
    expect(tokenLabel).toBe(`${Math.ceil(new TextEncoder().encode(wrapper).length / 4)} estimated tokens`);
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identity);
    expect((await request.post(`${provider}/control/${upstream.id}/fragment`, { data: { text: "The lantern flickered." } })).status()).toBe(200);
    expect((await request.post(`${provider}/control/${upstream.id}/finish`, { data: { reason: "stop" } })).status()).toBe(200);
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
    await expectSaved(page);
  });
}
