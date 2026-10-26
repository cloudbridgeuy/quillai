import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`tree lists a novel and a loose piece in one universe at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const prefix = `Story tree ${width}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: `${prefix} series`, position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: `${prefix} novel`, position: 0 });
    const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
      title: `${prefix} chapter`, position: 0, kind: "chapter", content: { ops: [{ insert: "Novel chapter text.\n" }] },
    });
    const container = await (await request.get(`${api}/containers/${series.id}`)).json();
    const piece = await post(`${api}/universes/${container.universeId}/pieces`, {
      title: `${prefix} loose piece`, position: 0, label: "short_story", shape: "leaf",
    });

    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/?${new URLSearchParams({
      apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
    })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await editor.click();
    await page.keyboard.press("Control+[");
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const chapterButton = outline.getByRole("button", { name: `${prefix} chapter`, exact: true });
    const pieceButton = outline.getByRole("button", { name: `${prefix} loose piece`, exact: true });
    await expect(chapterButton).toBeVisible();
    await expect(pieceButton).toBeVisible();

    await pieceButton.click();
    await expect(page.locator(".editor-chapter-title")).toHaveText(`${prefix} loose piece`);
    await expectEditorReady(editor);
    await expect(pieceButton).toHaveAttribute("aria-pressed", "true");
    await editor.click();
    await page.keyboard.type("Loose piece text.");
    await expect.poll(async () => {
      const detail = await (await request.get(`${api}/pieces/${piece.id}`)).json();
      return detail.content.ops.map((op: { insert: string }) => op.insert).join("");
    }).toBe("Loose piece text.\n");

    await page.reload();
    await expectEditorReady(editor);
    await expect(page.locator(".editor-chapter-title")).toHaveText(`${prefix} loose piece`);
    await expect(editor).toHaveText("Loose piece text.");
    expect(new URL(page.url()).searchParams.get("pieceId")).toBe(String(piece.id));

    await editor.click();
    await page.keyboard.press("Control+[");
    await outline.getByRole("button", { name: `${prefix} chapter`, exact: true }).click();
    await expect(page.locator(".editor-chapter-title")).toHaveText(`${prefix} chapter`);
    await expectEditorReady(editor);
    await expect(editor).toHaveText("Novel chapter text.");
  });
}
