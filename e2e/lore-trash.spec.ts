import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`deleted lore stays out of the panel and AI requests at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const series = await (await request.post(`${api}/series`, { data: { title: `Lore trash ${width}`, position: 0 } })).json();
    const base = `${api}/series/${series.id}`;
    const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
    const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
      title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Mira met Rowan.\n" }] },
    } })).json();
    const deletedText = "Mira is the navigator.";
    const activeText = "Rowan is the captain.";
    const deleted = await (await request.post(`${base}/lore`, { data: {
      kind: "character", name: "Mira", visible_text: deletedText, pinned: true,
    } })).json();
    expect((await request.post(`${base}/lore`, { data: {
      kind: "character", name: "Rowan", visible_text: activeText, pinned: true,
    } })).status()).toBe(201);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    const bodies: string[] = [];
    await page.route("**/ai/generate", async route => {
      bodies.push(route.request().postData() ?? "");
      await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
    });
    const captureRequests = async () => {
      const start = bodies.length;
      await editor.click();
      await page.keyboard.press("Control+End");
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      await expect.poll(() => bodies.length).toBe(start + 1);
      await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeEnabled();
      await editor.selectText();
      await page.getByRole("button", { name: "Edit selection", exact: true }).click();
      await page.getByRole("button", { name: "Apply", exact: true }).click();
      await expect.poll(() => bodies.length).toBe(start + 2);
      expect(JSON.parse(bodies[start + 1]!).action).toBe("edit");
      return bodies.slice(start);
    };
    for (const body of await captureRequests()) {
      expect(body).toContain(deletedText);
      expect(body).toContain(activeText);
    }
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const lore = page.getByRole("region", { name: "Series lore", exact: true });
    const entry = lore.getByRole("button", { name: "Mira · character · pinned", exact: true });
    await entry.click();
    const edit = page.getByRole("dialog", { name: "Edit lore", exact: true });
    await edit.getByRole("button", { name: "Delete lore", exact: true }).click();
    await edit.getByRole("button", { name: "Confirm delete", exact: true }).click();
    await expect(entry).toHaveCount(0);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(entry).toHaveCount(0);
    await expect(lore.getByRole("button", { name: "Rowan · character · pinned", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    for (const body of await captureRequests()) {
      expect(body).not.toContain(deletedText);
      expect(body).toContain(activeText);
    }
    expect((await request.post(`${base}/lore/${deleted.id}/restore`)).status()).toBe(200);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(entry).toBeVisible();
    await entry.click();
    await edit.getByRole("button", { name: "Delete lore", exact: true }).click();
    await edit.getByRole("button", { name: "Confirm delete", exact: true }).click();
    expect((await request.delete(`${base}/lore/${deleted.id}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${base}/lore/${deleted.id}/restore`)).status()).toBe(404);
  });
}
