import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`series and book trash hides descendants and restores chapter text at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object, status = 201) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(status);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: `Trash series ${width}`, position: 0 });
    const seriesUrl = `${api}/series/${series.id}`;
    const book = await post(`${seriesUrl}/books`, { title: "Trash novel", position: 0 });
    const bookUrl = `${seriesUrl}/books/${book.id}`;
    const chapter = await post(`${bookUrl}/chapters`, {
      title: "Trash chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Restored chapter text.\n" }] },
    });
    const chapterUrl = `${bookUrl}/chapters/${chapter.id}`;
    const container = await (await request.get(`${api}/containers/${series.id}`)).json();
    const anchor = await post(`${api}/universes/${container.universeId}/pieces`, {
      title: "Anchor piece", position: 0, label: "short_story", shape: "leaf",
    });
    await post(`${api}/pieces/${anchor.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Anchor text.\n" }] } }, 200);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(anchor.id) })}`);
    const editor = page.locator(".ql-editor");
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const novel = outline.locator("summary").filter({ has: page.getByRole("button", { name: "Rename 'Trash novel'", exact: true }) });
    await expect(novel).toBeVisible();
    await expect(outline.getByRole("button", { name: "Trash chapter", exact: true })).toBeVisible();
    expect((await request.delete(seriesUrl)).status()).toBe(204);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(novel).toHaveCount(0);
    await expect(outline.getByRole("button", { name: "Trash chapter", exact: true })).toHaveCount(0);
    expect((await request.post(`${seriesUrl}/restore`)).status()).toBe(200);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await outline.getByRole("button", { name: "Trash chapter", exact: true }).click();
    await expectEditorReady(editor);
    await expect(editor).toContainText("Restored chapter text.");
    expect((await request.delete(bookUrl)).status()).toBe(204);
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(anchor.id) })}`);
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(novel).toHaveCount(0);
    await expect(outline.getByRole("button", { name: "Trash chapter", exact: true })).toHaveCount(0);
    expect((await request.post(`${chapterUrl}/restore`)).status()).toBe(200);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(novel).toBeVisible();
    await outline.getByRole("button", { name: "Trash chapter", exact: true }).click();
    await expectEditorReady(editor);
    await expect(editor).toContainText("Restored chapter text.");
    expect((await request.delete(bookUrl)).status()).toBe(204);
    expect((await request.delete(`${bookUrl}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${bookUrl}/restore`)).status()).toBe(404);
    expect((await request.delete(seriesUrl)).status()).toBe(204);
    expect((await request.delete(`${seriesUrl}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${seriesUrl}/restore`)).status()).toBe(404);
  });

  test(`universe trash hides its pieces and lore from the browser and Continue at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object, status = 201) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(status);
      return response.json();
    };
    const universe = await post(`${api}/universes`, { title: `Trash universe ${width}`, position: 0 });
    const universeUrl = `${api}/universes/${universe.id}`;
    const piece = await post(`${universeUrl}/pieces`, { title: "Hidden story", position: 0, label: "short_story", shape: "leaf" });
    await post(`${api}/pieces/${piece.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Hidden story text.\n" }] } }, 200);
    await post(`${universeUrl}/lore`, { kind: "character", name: "Hidden Mira", visible_text: "Hidden universe lore.", pinned: true });
    const active = await post(`${api}/universes`, { title: `Active universe ${width}`, position: 1 });
    const anchor = await post(`${api}/universes/${active.id}/pieces`, { title: "Active story", position: 0, label: "short_story", shape: "leaf" });
    await post(`${api}/pieces/${anchor.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Active story text.\n" }] } }, 200);
    await post(`${api}/universes/${active.id}/lore`, { kind: "character", name: "Active Rowan", visible_text: "Active universe lore.", pinned: true });
    const bodies: string[] = [];
    await page.route("**/ai/generate", async route => {
      bodies.push(route.request().postData() ?? "");
      await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
    });
    const continueOnce = async () => {
      const count = bodies.length;
      await editor.click();
      await page.keyboard.press("Control+End");
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      await expect.poll(() => bodies.length).toBe(count + 1);
      await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeEnabled();
      return bodies[count]!;
    };
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    expect(await continueOnce()).toContain("Hidden universe lore.");
    expect((await request.delete(universeUrl)).status()).toBe(204);
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(anchor.id) })}`);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expect(outline.getByText(`Trash universe ${width}`, { exact: true })).toHaveCount(0);
    await expect(outline.getByRole("button", { name: "Hidden story", exact: true })).toHaveCount(0);
    const lore = page.getByRole("region", { name: "Series lore", exact: true });
    await expect(lore).not.toContainText("Hidden Mira");
    await expect(lore).toContainText("Active Rowan");
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const activeBody = await continueOnce();
    expect(activeBody).not.toContain("Hidden universe lore.");
    expect(activeBody).toContain("Active universe lore.");
    expect((await request.post(`${universeUrl}/restore`)).status()).toBe(200);
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
    await page.reload();
    await expectEditorReady(editor);
    await expect(editor).toContainText("Hidden story text.");
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(outline.getByRole("button", { name: "Hidden story", exact: true })).toBeVisible();
    await expect(lore).toContainText("Hidden Mira");
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    expect(await continueOnce()).toContain("Hidden universe lore.");
    expect((await request.delete(universeUrl)).status()).toBe(204);
    expect((await request.delete(`${universeUrl}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${universeUrl}/restore`)).status()).toBe(404);
  });
}
