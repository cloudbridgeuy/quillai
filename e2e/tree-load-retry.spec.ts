import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`tree load failure offers Try loading again and recovers at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Missing QUILLAI_E2E_API_URL");
    const post = async (path: string, data: object) => {
      const response = await request.post(`${api}${path}`, { data });
      expect(response.status(), await response.text()).toBe(201);
      return response.json();
    };
    const prefix = `Tree retry ${testInfo.project.name} ${width}`;
    const series = await post("/series", { title: `${prefix} series`, position: 0 });
    const book = await post(`/series/${series.id}/books`, { title: `${prefix} book`, position: 0 });
    const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, { title: `${prefix} chapter`, position: 0, kind: "chapter" });
    await page.setViewportSize({ width, height: 1000 });
    await page.route(`${api}/tree`, route => route.fulfill({
      status: 503, json: { error: "Could not load manuscript.", code: "INTERNAL_ERROR" },
    }), { times: 1 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const retry = outline.getByRole("button", { name: "Try loading again", exact: true });
    await expect(retry).toBeVisible();
    await expect(outline.getByRole("button", { name: "Retry manuscript", exact: true })).toHaveCount(0);
    await retry.click();
    await expect(outline.getByRole("button", { name: chapter.title, exact: true })).toBeVisible();
    await expect(retry).toHaveCount(0);
  });
}
