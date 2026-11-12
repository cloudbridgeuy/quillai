import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

for (const configured of [true, false]) {
  test(`Continue ${configured ? "persists text with" : "reports empty output without"} reasoning effort`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
    const identity = `GUZ1008-${crypto.randomUUID()}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: "Reasoning series", position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: "Reasoning book", position: 0 });
    const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
    const chapter = await post(chapters, { title: "Reasoning chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `Opening ${identity}\n` }] } });
    const before = await (await request.get(`${chapters}/${chapter.id}`)).json();
    const model = configured ? "e2e-reasoning" : "e2e-no-reasoning";
    await page.setViewportSize({ width: 1100, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await expectSaved(page);
    // Stands in for the server resolving a Settings model: requests carry none.
    await page.route(`${api}/ai/generate`, route => route.continue({ postData: JSON.stringify({ ...route.request().postDataJSON(), model }) }));
    await editor.click();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model } })).status()).toBe(200);
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    if (!configured) {
      await expect(page.locator('.editor-scene-notice[role="status"]')).toHaveText("The model used its whole budget without writing any text. Try again or pick another model.");
      await expect(editor).toHaveText(`Opening ${identity}`);
      const saved = await request.get(`${chapters}/${chapter.id}`);
      expect(saved.status()).toBe(200);
      expect(await saved.json()).toEqual(before);
      return;
    }
    await expect(editor).toContainText("Reasoned continuation.");
    await expectSaved(page);
    await expect.poll(async () => {
      const saved = await request.get(`${chapters}/${chapter.id}`);
      expect(saved.status()).toBe(200);
      return JSON.stringify((await saved.json()).content);
    }).toContain("Reasoned continuation.");
  });
}
