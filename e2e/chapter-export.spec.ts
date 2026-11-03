import { readFileSync } from "node:fs";
import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`chapter downloads at ${width}px`, async ({ page, request, browserName }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: `Export ${browserName} ${width}`, position: 0 });
    const base = `${api}/series/${series.id}`;
    const book = await post(`${base}/books`, { title: "Export book", position: 0 });
    const chapter = await post(`${base}/books/${book.id}/chapters`, {
      title: "Export chapter", position: 0, kind: "chapter",
      content: { ops: [{ insert: "Bold", attributes: { bold: true } }, { insert: " words\n" }] },
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    for (const [name, filename] of [["Download Markdown", "export-chapter.md"], ["Download text", "export-chapter.txt"], ["Download backup", "chapter-local.json"]]) {
      const link = page.getByRole("link", { name, exact: true });
      await expect(link).toBeVisible();
      const downloading = page.waitForEvent("download");
      await link.click();
      const download = await downloading;
      expect(download.suggestedFilename()).toBe(filename);
      if (name === "Download Markdown") {
        const content = readFileSync((await download.path())!, "utf8");
        expect(content).toContain("# Export chapter\n");
        expect(content).toContain("**Bold** words");
      }
    }
  });
}
