import { readFileSync } from "node:fs";
import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

function zipFile(bytes: Buffer, path: string): string {
  let offset = 0;
  while (bytes.readUInt32LE(offset) === 0x04034b50) {
    const size = bytes.readUInt32LE(offset + 18);
    const nameLength = bytes.readUInt16LE(offset + 26);
    const start = offset + 30 + nameLength + bytes.readUInt16LE(offset + 28);
    if (bytes.subarray(offset + 30, offset + 30 + nameLength).toString() === path) return bytes.subarray(start, start + size).toString();
    offset = start + size;
  }
  throw new Error(`Missing ${path}`);
}

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
    for (const [name, filename] of [["Download Markdown", "export-chapter.md"], ["Download text", "export-chapter.txt"], ["Download Word", "export-chapter.docx"], ["Download backup", "chapter-local.json"]]) {
      const link = page.getByRole("link", { name, exact: true });
      await expect(link).toBeVisible();
      const downloading = page.waitForEvent("download");
      await link.click();
      const download = await downloading;
      expect(download.suggestedFilename()).toBe(filename);
      if (name === "Download Word") {
        const content = readFileSync((await download.path())!);
        expect(content.subarray(0, 4).toString("hex")).toBe("504b0304");
        expect(zipFile(content, "word/document.xml")).toContain("Export chapter");
      }
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
      for (const [name, filename] of [["Markdown", "export-book.md"], ["text", "export-book.txt"], ["Word", "export-book.docx"], ["EPUB", "export-book.epub"]]) {
        const downloading = page.waitForEvent("download");
        await page.getByRole("link", { name, exact: true }).click();
        const download = await downloading;
        expect(download.suggestedFilename()).toBe(filename);
        const bytes = readFileSync((await download.path())!);
        const content = bytes.toString("utf8");
        if (name === "EPUB") {
          expect(bytes.subarray(0, 4).toString("hex")).toBe("504b0304");
          expect(bytes.subarray(38, 58).toString()).toBe("application/epub+zip");
          expect(zipFile(bytes, "mimetype")).toBe("application/epub+zip");
          expect(zipFile(bytes, "META-INF/container.xml")).toContain("OEBPS/content.opf");
          expect(zipFile(bytes, "OEBPS/chapter-1.xhtml")).toContain("unsaved live text");
          expect(zipFile(bytes, "OEBPS/chapter-2.xhtml")).toContain("Second chapter text");
          continue;
        }
        if (name === "Word") {
          expect(bytes.subarray(0, 4).toString("hex")).toBe("504b0304");
          const xml = zipFile(bytes, "word/document.xml");
          expect(xml).toContain("unsaved live text");
          expect(xml).toContain("Second chapter text");
          expect(xml.indexOf("Export chapter")).toBeLessThan(xml.indexOf("Second chapter"));
          continue;
        }
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
