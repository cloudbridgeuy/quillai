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
    await post(`${base}/books/${book.id}/chapters`, {
      title: "Second chapter", position: 1, kind: "chapter",
      content: { ops: [{ insert: "Second chapter text\n" }] },
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
    let release!: () => void;
    const gate = new Promise<void>(resolve => { release = resolve; });
    let sawPost!: () => void;
    const held = new Promise<void>(resolve => { sawPost = resolve; });
    await page.route(`${base}/books/${book.id}/chapters/${chapter.id}/deltas`, async route => {
      if (route.request().method() !== "POST") return route.continue();
      sawPost();
      await gate;
      await route.abort("aborted");
    });
    try {
      await page.locator(".ql-editor").click();
      await page.keyboard.press("ControlOrMeta+End");
      await page.keyboard.insertText(" unsaved live text");
      await held;
      for (const [name, filename] of [["Markdown", "export-book.md"], ["text", "export-book.txt"]]) {
        const downloading = page.waitForEvent("download");
        await page.getByRole("link", { name, exact: true }).click();
        const download = await downloading;
        expect(download.suggestedFilename()).toBe(filename);
        const content = readFileSync((await download.path())!, "utf8");
        expect(content).toContain("unsaved live text");
        expect(content).toContain("Second chapter text");
        const firstHeading = name === "Markdown" ? "## Export chapter\n" : "\n\nExport chapter\n";
        const secondHeading = name === "Markdown" ? "## Second chapter\n" : "\n\nSecond chapter\n";
        expect(content).toContain(firstHeading);
        expect(content).toContain(secondHeading);
        expect(content.indexOf(firstHeading)).toBeLessThan(content.indexOf(secondHeading));
      }
      await page.route(url => url.pathname === `/series/${series.id}/books/${book.id}/chapters` && url.searchParams.get("content") === "true", route => route.fulfill({ status: 500, body: "error" }));
      let downloads = 0;
      page.on("download", () => { downloads++; });
      await page.getByRole("link", { name: "Markdown", exact: true }).click();
      await expect(page.getByRole("alert").filter({ hasText: "Could not export book" })).toBeVisible();
      expect(downloads).toBe(0);
    } finally {
      release();
      await page.unrouteAll({ behavior: "wait" });
    }
  });
}
