import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  for (const action of ["load", "select", "create"] as const) {
    test(`active chapter scrolls inside the tree on ${action} at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const prefix = `Scroll ${width} ${action}`;
      const post = async (url: string, data: object) => {
        const response = await request.post(url, { data });
        expect(response.status()).toBe(201);
        return response.json();
      };
      for (let position = 0; position < 6; position++) {
        const series = await post(`${api}/series`, { title: `${prefix} filler ${position}`, position });
        const book = await post(`${api}/series/${series.id}/books`, { title: "Filler book", position: 0 });
        await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
          title: "Filler chapter", position: 0, kind: "chapter",
        });
      }
      const series = await post(`${api}/series`, { title: prefix, position: 1000 });
      const book = await post(`${api}/series/${series.id}/books`, { title: `${prefix} book`, position: 0 });
      const chaptersUrl = `${api}/series/${series.id}/books/${book.id}/chapters`;
      const chapters = [];
      for (let position = 0; position < 12; position++) {
        chapters.push(await post(chaptersUrl, {
          title: `${prefix} chapter ${position}`, position, kind: "chapter",
          content: { ops: [{ insert: `Opening ${position} text.\n` }] },
        }));
      }
      const initial = action === "select" ? chapters[0] : chapters.at(-1);
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(`/?${new URLSearchParams({
        apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(initial.id),
      })}`);
      const editor = page.locator(".ql-editor");
      await expectEditorReady(editor);
      await editor.click();
      await editor.evaluate(element => {
        const text = element.querySelector("p")!.firstChild!;
        const range = document.createRange();
        range.setStart(text, 0);
        range.setEnd(text, 7);
        const selection = window.getSelection()!;
        selection.removeAllRanges();
        selection.addRange(range);
      });
      const pageScroll = await page.evaluate(() => ({ x: scrollX, y: scrollY }));
      await page.keyboard.press("Control+[");
      const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
      const tree = outline.locator(".editor-manuscript-tree");
      await expect(tree).toBeVisible();
      let title = initial.title;
      if (action === "load") {
        await expect(editor).toBeFocused();
        expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
      } else if (action === "select") {
        title = chapters.at(-1).title;
        await outline.evaluate(element => { element.scrollTop = 0; });
        const target = outline.getByRole("button", { name: title, exact: true });
        await expect(target).not.toBeInViewport();
        await outline.focus();
        await target.evaluate(element => element.click());
        await expect(page.locator(".editor-chapter-title")).toHaveText(title);
        await expectEditorReady(editor);
        await expect(outline).toBeFocused();
        await expect(editor).toHaveText("Opening 11 text.");
      } else {
        const seriesRow = tree.locator(":scope > ul > li").filter({ has: page.getByText(prefix, { exact: true }) });
        await seriesRow.getByRole("button", { name: "+ Chapter", exact: true }).click();
        const field = outline.getByRole("textbox", { name: "Chapter title", exact: true });
        title = `${prefix} new chapter`;
        await field.fill(title);
        let release!: () => void;
        let intercepted!: () => void;
        const held = new Promise<void>(resolve => { release = resolve; });
        const requested = new Promise<void>(resolve => { intercepted = resolve; });
        await page.route(chaptersUrl, async route => {
          if (route.request().method() !== "POST") return route.continue();
          const response = await route.fetch();
          intercepted();
          await held;
          await route.fulfill({ response });
        });
        await field.press("Enter");
        await requested;
        await outline.evaluate(element => { element.scrollTop = 0; });
        release();
        await expect(page.locator(".editor-chapter-title")).toHaveText(title);
        await expectEditorReady(editor);
        await expect(editor).toHaveText("");
        await expect(outline.getByRole("textbox")).toHaveCount(0);
      }
      const active = outline.getByRole("button", { name: title, exact: true });
      await expect(active).toHaveAttribute("aria-pressed", "true");
      await expect(active).toBeInViewport({ ratio: 1 });
      await expect(active).not.toBeFocused();
      expect(await outline.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
      expect(await page.evaluate(() => ({ x: scrollX, y: scrollY }))).toEqual(pageScroll);
      expect(await page.locator("#manuscript-drawer").evaluate(element => element.scrollTop)).toBe(0);
      await page.keyboard.press("Escape");
      await expect(editor).toBeFocused();
      if (action === "load") expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
    });
  }
}
