import { expect, test, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const friendly = "Couldn't reach your manuscript's storage. Your text is still here; try again.";

async function setup(request: APIRequestContext) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const seriesResponse = await request.post(`${api}/series`, { data: { title: "Server errors", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const response = await request.post(chaptersUrl, { data: { title: "First chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } } });
  expect(response.status()).toBe(201);
  const chapter = await response.json();
  const query = new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
  return { chapterUrl: `${chaptersUrl}/${chapter.id}`, query };
}

for (const width of [1100, 1440]) {
  test(`chapter load hides server details at ${width}px`, async ({ page, request }) => {
    const { chapterUrl, query } = await setup(request);
    await page.setViewportSize({ width, height: 1000 });
    await page.route(chapterUrl, async route => {
      if (route.request().method() !== "GET") return route.continue();
      await route.fulfill({ status: 500, json: { error: "sqlx error", code: "SQLX" } });
    });
    await page.goto(`/?${query}`);
    await expect(page.getByText(friendly, { exact: false })).toBeVisible();
    await expect(page.getByText("sqlx error", { exact: false })).not.toBeVisible();
  });

  test(`lore save hides server details at ${width}px`, async ({ page, request }) => {
    const { query } = await setup(request);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${query}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const lore = page.getByRole("region", { name: "Series lore", exact: true });
    await lore.getByRole("button", { name: "New lore", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Create lore", exact: true });
    await dialog.getByLabel("Name", { exact: true }).fill("Mira");
    await page.route("**/universes/*/lore", async route => {
      if (route.request().method() !== "POST") return route.continue();
      await route.fulfill({ status: 500, json: { error: "sqlx error", code: "SQLX" } });
    });
    await dialog.getByRole("button", { name: "Save lore", exact: true }).click();
    await expect(dialog.getByText(friendly, { exact: true })).toBeVisible();
    await expect(page.getByText("sqlx error", { exact: false })).not.toBeVisible();
    await expect(dialog.getByLabel("Name", { exact: true })).toHaveValue("Mira");
  });
}
