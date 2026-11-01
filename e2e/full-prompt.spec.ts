import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test.afterEach(async ({ request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (api) expect((await request.put(`${api}/settings/author-guidance`, { data: { text: "" } })).status()).toBe(200);
});

for (const width of [1100, 1440]) {
  test(`full prompt shows the current request at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    expect((await request.put(`${api}/settings/author-guidance`, { data: { text: "Keep the tone hopeful." } })).status()).toBe(200);
    const series = await (await request.post(`${api}/series`, { data: { title: `Full prompt ${width}`, position: 0 } })).json();
    const base = `${api}/series/${series.id}`;
    const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
    const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
      title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Mira arrived.\n" }] },
    } })).json();
    expect((await request.post(`${base}/lore`, { data: {
      kind: "character", name: "Mira", visible_text: "Mira is the navigator.", level: "known_dont_reveal",
    } })).status()).toBe(201);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const inspector = page.locator("#context-drawer");
    await expect(inspector.getByRole("textbox", { name: "Lore prompt section", exact: true })).toHaveValue(/Mira is the navigator/);
    await inspector.getByRole("button", { name: "Full prompt", exact: true }).click();
    const modal = page.getByRole("dialog", { name: "Full prompt", exact: true });
    await expect(modal).toBeVisible();
    await expect(modal).toContainText("Keep the tone hopeful.");
    await expect(modal).toContainText("Mira is the navigator.");
    await expect(modal).toContainText("Mira arrived.");
    const text = await modal.innerText();
    expect(text.indexOf("Keep the tone hopeful.")).toBeGreaterThanOrEqual(0);
    expect(text.indexOf("Mira is the navigator.")).toBeGreaterThan(text.indexOf("Keep the tone hopeful."));
    expect(text.indexOf("Mira arrived.")).toBeGreaterThan(text.indexOf("Mira is the navigator."));
    await expect(modal.getByRole("textbox")).toHaveCount(0);
    await expect(modal.locator('input, textarea, [contenteditable]:not([contenteditable="false"])')).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(modal).toBeHidden();
    await expect(inspector).toBeVisible();
    await expect(inspector.getByRole("button", { name: "Full prompt", exact: true })).toBeFocused();
  });
}
