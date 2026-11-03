import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`empty book offers chapter creation at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const seriesResponse = await request.post(`${api}/series`, { data: { title: `Empty book ${width}`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${api}/series/${series.id}/books`, { data: { title: "Empty book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    await page.route(`${api}/series`, async route => {
      if (route.request().method() !== "GET") return route.continue();
      await route.fulfill({ json: [series] });
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
    const notice = page.locator('.editor-scene-notice[data-tone="info"][role="status"]');
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("No chapters yet.");
    await expect(page.getByRole("button", { name: "Create chapter", exact: true })).toHaveCount(0);
    await notice.getByRole("button", { name: "Create the first chapter", exact: true }).click();
    await expectEditorReady(page.locator(".ql-editor"));
    await expect(page.locator(".editor-chapter-title")).toHaveText("Chapter One");
    await expect(notice).toHaveCount(0);
  });
}
