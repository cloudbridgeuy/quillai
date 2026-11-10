import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openChapter(page: Page, request: APIRequestContext, width: number) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Hints", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Hints", position: 0 } })).json();
  const chapter = await (await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Hints", position: 0, kind: "chapter", content: { ops: [{ insert: "A quiet harbour.\n" }] },
  } })).json();
  await page.setViewportSize({ width, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  return { apiUrl, seriesId: series.id as number };
}

for (const width of [1100, 1440]) {
  test(`Hints language saves to the series at ${width}px`, async ({ page, request }) => {
    const { apiUrl, seriesId } = await openChapter(page, request, width);
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings" });
    await expect(settings.getByRole("heading", { name: "Hints" })).toBeVisible();
    const language = settings.getByLabel("Language", { exact: true });
    await expect(language).toHaveValue("en");
    await language.selectOption("es");
    await expect.poll(async () => (await (await request.get(`${apiUrl}/series/${seriesId}/rules`)).json()).language).toBe("es");
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(settings.getByLabel("Language", { exact: true })).toHaveValue("es");
  });

  test(`a failed Hints save shows the error and restores the language at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await page.route("**/series/*/rules", (route) => route.request().method() === "PUT"
      ? route.fulfill({ status: 422, json: { error: "bad", code: "VALIDATION_ERROR" } })
      : route.fallback());
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings" });
    await settings.getByLabel("Language", { exact: true }).selectOption("es");
    await expect(settings.getByRole("alert")).toBeVisible();
    await expect(settings.getByLabel("Language", { exact: true })).toHaveValue("en");
  });
}

test("no Hints section below 1100px", async ({ page, request }) => {
  await openChapter(page, request, 1099);
  await expect(page.getByRole("heading", { name: "Hints" })).toHaveCount(0);
});
