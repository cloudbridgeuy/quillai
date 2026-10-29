import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openEditor(page: Page, request: APIRequestContext, width: number) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Settings", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Settings", position: 0 } })).json();
  const chapter = await (await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Settings", position: 0, kind: "chapter", content: { ops: [{ insert: "A quiet harbour.\n" }] },
  } })).json();
  await page.setViewportSize({ width, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
}

for (const width of [1100, 1440]) {
  test(`settings has no close button at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await expect(page.getByRole("dialog", { name: "Settings" }).getByRole("button", { name: /close/i })).toHaveCount(0);
  });

  test(`settings gear toggles at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "Settings" })).toBeHidden();
  });

  test(`Escape dismisses settings at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: "Settings" })).toBeHidden();
  });

  test(`outside click dismisses settings at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await page.locator(".editor-chapter-title").click();
    await expect(page.getByRole("dialog", { name: "Settings" })).toBeHidden();
  });

  test(`settings rows have equal gaps at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    const settings = page.getByRole("dialog", { name: "Settings" });
    const rows = [settings.getByRole("heading", { name: "Settings" }).locator(".."), ...["Window Size", "Whole document", "Show Continue window", "Tint AI text"].map((name) => settings.getByLabel(name, { exact: true }).locator(".."))];
    const boxes = await Promise.all(rows.map((row) => row.boundingBox()));
    if (boxes.some((box) => !box)) throw new Error("Missing settings row");
    const gaps = boxes.slice(1).map((box, index) => box!.y - (boxes[index]!.y + boxes[index]!.height));
    for (const gap of gaps) {
      expect(gap).toBeGreaterThan(0);
      expect(Math.abs(gap - gaps[0]!)).toBeLessThanOrEqual(1);
    }
  });
}
