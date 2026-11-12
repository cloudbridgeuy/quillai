import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectSaved } from "./save-state";

async function setup(page: Page, request: APIRequestContext) {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identity = `GUZ1568-${crypto.randomUUID()}`;
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Continue icon series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Continue icon book", position: 0 });
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Continue icon chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `Opening line. ${identity}\n` }] },
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectSaved(page);
  return { identity, provider };
}

for (const width of [1100, 1440]) {
  test(`Continue is an icon, Stop swaps into its slot, and a cut-off reply badges it at ${width}px`, async ({ page, request }) => {
    await page.setViewportSize({ width, height: 1000 });
    const { identity, provider } = await setup(page, request);
    const button = page.locator(".editor-continue");
    await expect(button).toHaveCount(1);
    await expect(button).toHaveAccessibleName("Continue");
    await expect(button).toHaveText("➤");
    await expect(button.getByRole("img")).toHaveCount(0);
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
    await button.click();
    await expect(button).toHaveAccessibleName("Stop AI");
    await expect(button).toHaveText("■");
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identity);
    const control = `${provider}/control/${upstream.id}`;
    expect((await request.post(`${control}/fragment`, { data: { text: "A cut-off reply." } })).status()).toBe(200);
    expect((await request.post(`${control}/finish`, { data: { reason: "length" } })).status()).toBe(200);
    await expect(button).toHaveAccessibleName("Continue");
    await expect(button).toHaveText("➤");
    const badge = button.getByRole("img", { name: "from a cut-off reply", exact: true });
    await expect(badge).toBeVisible();
    await expect(badge).toHaveAttribute("title", "Continue writing from a cut-off reply");
  });
}
