import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import type { AiGenerateRequest } from "../app/src/md_editor/ai_contract";

async function setup(page: Page, request: APIRequestContext) {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identities = Array.from({ length: 4 }, (_, i) => `GUZ868-${i + 1}-${crypto.randomUUID()}`);
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Thread series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Thread book", position: 0 });
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const first = await post(chapters, { title: "First thread chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `First opening. ${identities.slice(0, 3).join(" ")}\n` }] } });
  await post(chapters, { title: "Second thread chapter", position: 1, kind: "chapter", content: { ops: [{ insert: `Second opening. ${identities[3]}\n` }] } });
  const requests: AiGenerateRequest[] = [];
  page.on("request", (outgoing) => {
    if (outgoing.url().endsWith("/ai/generate")) requests.push(outgoing.postDataJSON());
  });
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(first.id) })}`);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByText("Advanced AI controls", { exact: true }).click();
  await page.getByRole("combobox", { name: "Continue model", exact: true }).selectOption("e2e-continue");
  const run = async (index: number) => {
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity: identities[index], model: "e2e-continue" } })).status()).toBe(200);
    await page.getByRole("button", { name: /^Continue writing/ }).click();
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identities[index]);
    const control = `${provider}/control/${upstream.id}`;
    expect((await request.post(`${control}/fragment`, { data: { text: "A cut-off reply." } })).status()).toBe(200);
    expect((await request.post(`${control}/finish`, { data: { reason: "length" } })).status()).toBe(200);
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  };
  return { requests, run };
}

test("a cut-off reply shows the resume panel", async ({ page, request }) => {
  const { run } = await setup(page, request);
  await run(0);
  await expect(page.getByText("Continuing from a cut-off reply", { exact: true })).toBeVisible();
});

test("Continue sends the cut-off reply as messages", async ({ page, request }) => {
  const { requests, run } = await setup(page, request);
  await run(0);
  await run(1);
  expect(requests[1]!.messages).toEqual([
    { role: "user", content: "Continue the scene." },
    { role: "assistant", content: "A cut-off reply." },
  ]);
});

test("Discard clears the cut-off reply before Continue", async ({ page, request }) => {
  const { requests, run } = await setup(page, request);
  await run(0);
  await page.getByRole("button", { name: "Discard", exact: true }).click();
  await expect(page.getByText("Continuing from a cut-off reply", { exact: true })).toBeHidden();
  await run(1);
  expect(requests[1]!.messages).toEqual([]);
});

test("Continue in another chapter sends no first chapter messages", async ({ page, request }) => {
  const { requests, run } = await setup(page, request);
  await run(0);
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await page.getByRole("button", { name: "Second thread chapter", exact: true }).click();
  await expect(page.locator(".ql-editor")).toContainText("Second opening.");
  await run(3);
  expect(requests[1]!.messages).toEqual([]);
});
