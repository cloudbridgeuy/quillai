import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`tree rename saves titles and cancels drafts at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Missing QUILLAI_E2E_API_URL");
    const post = async (path: string, data: object) => {
      const response = await request.post(`${api}${path}`, { data });
      expect(response.status(), await response.text()).toBe(201);
      return response.json();
    };
    const prefix = `Rename ${testInfo.project.name} ${width}`;
    const series = await post("/series", { title: `${prefix} series`, position: 0 });
    const book = await post(`/series/${series.id}/books`, { title: `${prefix} book`, position: 0 });
    const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, { title: `${prefix} chapter`, position: 0, kind: "chapter" });
    const container = await (await request.get(`${api}/containers/${series.id}`)).json();
    const piece = await post(`/universes/${container.universeId}/pieces`, { title: `${prefix} piece`, position: 0, label: "short_story", shape: "leaf" });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const tree = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const edit = async (title: string) => {
      const button = tree.getByRole("button", { name: `Rename '${title}'`, exact: true });
      await button.locator("..").hover();
      await button.click();
      const field = tree.getByRole("textbox", { name: `Rename '${title}'`, exact: true });
      await expect(field).toBeFocused();
      return field;
    };
    let puts = 0;
    page.on("request", request => { if (request.method() === "PUT") puts++; });
    let field = await edit(chapter.title);
    await field.fill("Discard this");
    await field.press("Escape");
    await expect(field).toHaveCount(0);
    await expect(tree.getByRole("button", { name: chapter.title, exact: true })).toBeVisible();
    field = await edit(chapter.title);
    await field.fill("   ");
    await field.press("Enter");
    await expect(tree.getByRole("alert")).toHaveText("Enter a title.");
    await field.fill(`  ${chapter.title}  `);
    await field.press("Enter");
    await expect(field).toHaveCount(0);
    expect(puts).toBe(0);
    const items = [
      { ...series, url: `${api}/series/${series.id}` },
      { ...book, url: `${api}/series/${series.id}/books/${book.id}` },
      { ...chapter, url: `${api}/series/${series.id}/books/${book.id}/chapters/${chapter.id}` },
      { ...piece, url: `${api}/pieces/${piece.id}` },
    ];
    for (const [index, item] of items.entries()) {
      field = await edit(item.title);
      await field.fill(`  ${item.title} renamed  `);
      const saved = page.waitForResponse(response => response.url() === item.url && response.request().method() === "PUT");
      if (index === 1) await field.press("Tab");
      else await field.press("Enter");
      expect((await saved).status()).toBe(200);
      await expect(field).toHaveCount(0);
      await expect(tree.getByRole("button", { name: `Rename '${item.title} renamed'`, exact: true })).toHaveCount(1);
      expect((await (await request.get(item.url)).json()).title).toBe(`${item.title} renamed`);
    }
    await expect(page.locator(".editor-chapter-title")).toHaveText(`${chapter.title} renamed`);
    await expect(page).toHaveTitle(`${chapter.title} renamed — QuillAI`);
    expect(puts).toBe(4);
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await expect(page).toHaveTitle(`${chapter.title} renamed — QuillAI`);
  });
}
