import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test.use({ viewport: { width: 1100, height: 1000 } });

function apiUrl() {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  return api;
}

test("tab title without a chapter", async ({ page }) => {
  const api = apiUrl();
  await page.route(`${api}/series`, route => route.fulfill({ json: [] }));
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
  await expect(page.getByRole("button", { name: "Create the first chapter", exact: true })).toBeVisible();
  await expect(page).toHaveTitle("QuillAI");
});

test("tab title while loading and after a load error", async ({ page }) => {
  const api = apiUrl();
  let release!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  await page.route(`${api}/series`, async route => {
    await held;
    await route.fulfill({ status: 500, json: { error: "Chapter load failed" } });
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
  try {
    await expect(page.getByText("Loading chapter…", { exact: true })).toBeVisible();
    await expect(page).toHaveTitle("QuillAI");
  } finally {
    release();
  }
  await expect(page.getByText("Loading chapter…", { exact: true })).toHaveCount(0);
  await expect(page.locator(".ql-editor")).not.toHaveAttribute("contenteditable", "true");
  await expect(page).toHaveTitle("QuillAI");
});

test("tab title follows the open chapter and chapter switches", async ({ page, request }) => {
  const api = apiUrl();
  const seriesResponse = await request.post(`${api}/series`, { data: { title: "Tab titles", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapters = [];
  for (const [position, title] of ["First light", "Nightfall"].entries()) {
    const response = await request.post(chaptersUrl, { data: { title, position, kind: "chapter" } });
    expect(response.status()).toBe(201);
    chapters.push(await response.json());
  }
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapters[0].id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page).toHaveTitle("First light — QuillAI");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  let release!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  await page.route(`${chaptersUrl}/${chapters[1].id}`, async route => {
    await held;
    await route.continue();
  });
  await page.getByRole("region", { name: "Manuscript outline", exact: true }).getByRole("button", { name: "Nightfall", exact: true }).click();
  try {
    await expect(page.getByText("Loading chapter…", { exact: true })).toBeVisible();
    await expect(page).toHaveTitle("QuillAI");
  } finally {
    release();
  }
  await expect(page.locator(".editor-chapter-title")).toHaveText("Nightfall");
  await expect(page).toHaveTitle("Nightfall — QuillAI");
  expect((await request.put(`${chaptersUrl}/${chapters[0].id}`, { data: { title: "New dawn" } })).status()).toBe(200);
  await page.getByRole("region", { name: "Manuscript outline", exact: true }).getByRole("button", { name: "First light", exact: true }).click();
  await expect(page).toHaveTitle("New dawn — QuillAI");
});

test("favicon link serves an image", async ({ page, request }) => {
  const api = apiUrl();
  await page.route(`${api}/series`, route => route.fulfill({ json: [] }));
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
  const icon = page.locator('link[rel="icon"]');
  await expect(icon).toHaveCount(1);
  const href = await icon.getAttribute("href");
  expect(href).toBeTruthy();
  const response = await request.get(new URL(href!, page.url()).href);
  console.log(`favicon: ${response.status()} ${response.headers()["content-type"]}`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toMatch(/^image\//);
  expect(await response.text()).toContain("QuillAI quill");
});
