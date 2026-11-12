import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved, expectUnsaved } from "./save-state";

for (const width of [1100, 1440]) {
  for (const scenario of ["chapter save", "lore load", "lore save", "manuscript load"] as const) {
    test(`${scenario} error at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const seriesResponse = await request.post(`${api}/series`, { data: { title: "Error assertions", position: 0 } });
      expect(seriesResponse.status()).toBe(201);
      const series = await seriesResponse.json();
      const base = `${api}/series/${series.id}`;
      const bookResponse = await request.post(`${base}/books`, { data: { title: "Book", position: 0 } });
      expect(bookResponse.status()).toBe(201);
      const book = await bookResponse.json();
      const chapters = `${base}/books/${book.id}/chapters`;
      const chapterResponse = await request.post(chapters, { data: {
        title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] },
      } });
      expect(chapterResponse.status()).toBe(201);
      const chapter = await chapterResponse.json();
      const deltaUrl = `${chapters}/${chapter.id}/deltas`;
      await page.setViewportSize({ width, height: 1000 });
      if (scenario === "lore load") {
        await page.route("**/universes/*/lore", async (route) => {
          if (route.request().method() !== "GET") return route.continue();
          await route.fulfill({ status: 503, json: { error: "Unavailable", code: "UNAVAILABLE" } });
        });
      }
      await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
      const editor = page.locator(".ql-editor");
      await expectEditorReady(editor);
      const alert = scenario === "chapter save"
        ? page.getByRole("alert", { name: "Chapter save status", exact: true })
        : page.getByRole("alert").filter({ hasText: "Something went wrong. Try again." });
      if (scenario === "chapter save") {
        await expectSaved(page);
        await page.getByRole("button", { name: "Settings", exact: true }).click();
        for (const name of ["Whole document", "Show Continue window", "Tint AI text"]) {
          const toggle = page.getByRole("checkbox", { name, exact: true });
          const checked = await toggle.isChecked();
          await toggle.click();
          expect(await toggle.isChecked()).toBe(!checked);
        }
        await page.getByRole("button", { name: "Settings", exact: true }).click();
        let release!: () => void;
        const gate = new Promise<void>((resolve) => { release = resolve; });
        let started!: () => void;
        const pending = new Promise<void>((resolve) => { started = resolve; });
        await page.route(deltaUrl, async (route) => {
          if (route.request().method() !== "POST") return route.continue();
          started();
          await gate;
          await route.fulfill({ status: 503, json: { error: "Save unavailable", code: "UNAVAILABLE" } });
        });
        await editor.click();
        await page.keyboard.type(" edited");
        await pending;
        await expectUnsaved(page);
        release();
        await expect(alert.locator("span").first()).toHaveText("Not saved");
        await expect(alert.locator("span").first()).toBeVisible();
        await expect(alert.locator("xpath=ancestor::*[@role='status']")).toHaveCount(0);
      } else {
        if (scenario === "manuscript load") {
          await page.route(`${api}/tree`, async (route) => {
            if (route.request().method() !== "GET") return route.continue();
            await route.fulfill({ status: 503, json: { error: "Unavailable", code: "UNAVAILABLE" } });
          });
        }
        await page.getByRole("button", { name: "Manuscript", exact: true }).click();
        if (scenario === "lore save") {
          await page.route("**/universes/*/lore", async (route) => {
            if (route.request().method() !== "POST") return route.continue();
            await route.fulfill({ status: 503, json: { error: "Unavailable", code: "UNAVAILABLE" } });
          });
          await page.getByRole("region", { name: "Series lore" }).getByRole("button", { name: "New lore" }).click();
          const dialog = page.getByRole("dialog", { name: "Create lore" });
          const pin = dialog.getByRole("checkbox", { name: "Pin in AI context", exact: true });
          const checked = await pin.isChecked();
          await pin.click();
          expect(await pin.isChecked()).toBe(!checked);
          await dialog.getByLabel("Name", { exact: true }).fill("Mira");
          await dialog.getByRole("button", { name: "Save lore" }).click();
        }
      }
      await expect(alert).toBeVisible();
      const geometry = await alert.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        return { scrollWidth: element.scrollWidth, clientWidth: element.clientWidth,
          left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom,
          width: window.innerWidth, height: window.innerHeight };
      });
      expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.clientWidth);
      expect(geometry.left).toBeGreaterThanOrEqual(0);
      expect(geometry.right).toBeLessThanOrEqual(geometry.width);
      expect(geometry.top).toBeGreaterThanOrEqual(0);
      expect(geometry.bottom).toBeLessThanOrEqual(geometry.height);
    });
  }
}
