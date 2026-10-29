import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("failed chapter save resumes after the delta route is released", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const seriesResponse = await request.post(`${apiUrl}/series`, {
    data: { title: "Recovery series", position: 0 },
  });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
    data: { title: "Recovery book", position: 0 },
  });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: {
      title: "Recovery chapter", position: 0, kind: "chapter",
      content: { ops: [{ insert: "Opening line\n" }] },
    },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  const chapterUrl = `${chaptersUrl}/${chapter.id}`;
  const deltaUrl = `${chapterUrl}/deltas`;

  await page.setViewportSize({ width: 1100, height: 1000 });
  const query = new URLSearchParams({
    apiBaseUrl: apiUrl,
    seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
  });
  await page.goto(`/?${query}`);
  const editor = page.locator(".ql-editor");
  const status = page.getByRole("status", { name: "Chapter save status", exact: true });
  await expect(editor).toHaveText("Opening line");
  await expect(status).toHaveText("Saved");

  let releaseFailure!: () => void;
  const failureGate = new Promise<void>((resolve) => { releaseFailure = resolve; });
  let sawFailure!: () => void;
  const failing = new Promise<void>((resolve) => { sawFailure = resolve; });
  let posts = 0;
  await page.route(deltaUrl, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    posts++;
    sawFailure();
    await failureGate;
    await route.fulfill({ status: 503, json: { error: "Save unavailable", code: "UNAVAILABLE" } });
  });
  const failedSave = page.waitForResponse((response) => (
    response.url() === deltaUrl && response.request().method() === "POST" && response.status() === 503
  ));

  await editor.evaluate((element) => {
    (element as HTMLElement).focus();
    const text = element.querySelector("p")?.firstChild;
    if (!text?.textContent) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, text.textContent.length);
    range.collapse(true);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  });
  await page.keyboard.type(" before");
  await failing;
  await page.keyboard.type(" during");
  await expect(editor).toHaveText("Opening line before during");
  releaseFailure();
  await failedSave;
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true }).locator("span").first()).toHaveText("Not saved");
  expect(posts).toBe(1);

  await page.unroute(deltaUrl);
  const resumed = page.waitForResponse((response) => (
    response.url() === deltaUrl && response.request().method() === "POST" && response.ok()
  ), { timeout: 15_000 });
  await resumed;
  await expect(status).toHaveText("Saved");
  await expect(editor).toHaveText("Opening line before during");
  const savedResponse = await request.get(chapterUrl);
  expect(savedResponse.ok()).toBe(true);
  expect((await savedResponse.json()).content).toEqual({ ops: [{ insert: "Opening line before during\n" }] });
});
