import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`deleted chapters stay out of the tree and restore their text at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object, status = 201) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(status);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: `Trash series ${width}`, position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: "Trash book", position: 0 });
    const base = `${api}/series/${series.id}/books/${book.id}/chapters`;
    const chapters = [];
    for (const [position, title] of ["Before", "Deleted chapter", "After"].entries()) {
      chapters.push(await post(base, { title, position, kind: "chapter", content: { ops: [{ insert: `${title} text.\n` }] } }));
    }
    const deleted = `${base}/${chapters[1].id}`;
    await post(`${deleted}/deltas`, { baseRev: chapters[1].head_rev, delta: { ops: [{ insert: "Saved " }] } }, 200);
    const before = await (await request.get(deleted)).json();
    const revisions = await (await request.get(`${deleted}/deltas?since=0`)).json();
    expect(revisions.length).toBeGreaterThan(0);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapters[0].id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    expect((await request.delete(deleted)).status()).toBe(204);
    const remaining = await (await request.get(base)).json();
    expect(remaining.map((chapter: { id: number }) => chapter.id)).toEqual([chapters[0].id, chapters[2].id]);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expect(outline.getByRole("button", { name: "Deleted chapter", exact: true })).toHaveCount(0);
    for (const title of ["After", "Before"]) {
      await outline.getByRole("button", { name: title, exact: true }).click();
      await expect(page.locator(".editor-chapter-title")).toHaveText(title);
      await expectEditorReady(editor);
      await expect(editor).toContainText(`${title} text.`);
    }
    expect((await request.post(`${deleted}/restore`)).status()).toBe(200);
    await page.reload();
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await outline.getByRole("button", { name: "Deleted chapter", exact: true }).click();
    await expectEditorReady(editor);
    await expect(editor).toContainText("Saved Deleted chapter text.");
    const after = await (await request.get(deleted)).json();
    expect(after.head_rev).toBe(before.head_rev);
    expect(after.content).toEqual(before.content);
    expect((await (await request.get(`${deleted}/deltas?since=0`)).json()).length).toBe(revisions.length);
    expect((await request.delete(deleted)).status()).toBe(204);
    expect((await request.delete(`${deleted}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${deleted}/restore`)).status()).toBe(404);
  });

  test(`deleting one scene keeps the other scene in its script at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object, status = 201) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(status);
      return response.json();
    };
    const universe = await post(`${api}/universes`, { title: `Trash script ${width}`, position: 0 });
    const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Script", position: 0, label: "screenplay", shape: "group" });
    const one = await post(`${api}/pieces/${group.id}/children`, { title: "First scene", position: 0, label: "scene" });
    const two = await post(`${api}/pieces/${group.id}/children`, { title: "Second scene", position: 1, label: "scene" });
    await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "First scene text.\n" }] } }, 200);
    await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Second scene text.\n" }] } }, 200);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    expect((await request.delete(`${api}/pieces/${one.id}`)).status()).toBe(204);
    await page.reload();
    await expectEditorReady(editor);
    await expect(editor).not.toContainText("First scene text.");
    await expect(editor).toContainText("Second scene text.");
    await expect(editor.locator(".ql-scene-break")).toHaveCount(1);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expect(outline.getByRole("button", { name: "First scene", exact: true })).toHaveCount(0);
    await expect(outline.getByRole("button", { name: "Second scene", exact: true })).toBeVisible();
    expect((await request.delete(`${api}/pieces/${one.id}?permanent=true`)).status()).toBe(204);
    expect((await request.post(`${api}/pieces/${one.id}/restore`)).status()).toBe(404);
  });
}
