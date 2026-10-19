import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("two pages save, sync, and reload one chapter", async ({ context, page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const seriesResponse = await request.post(`${apiUrl}/series`, {
    data: { title: "E2E series", position: 0 },
  });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
    data: { title: "E2E book", position: 0 },
  });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: {
      title: "E2E chapter", position: 0, kind: "chapter",
      content: { ops: [{ insert: "Opening line\n" }] },
    },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  const chapterUrl = `${chaptersUrl}/${chapter.id}`;
  const query = new URLSearchParams({
    apiBaseUrl: apiUrl,
    seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
  });
  const second = await context.newPage();
  const failures: string[] = [];
  for (const tab of [page, second]) {
    tab.on("pageerror", (error) => failures.push(error.message));
    tab.on("response", (response) => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.url()}`);
      }
    });
    await tab.goto(`/?${query}`);
    await expect(tab.locator(".ql-editor")).toHaveText("Opening line");
    await expect(tab.getByRole("status").filter({ hasText: "E2E chapter" })).toHaveText("E2E chapter · Saved");
  }

  await page.locator(".ql-editor").fill("Page one writes");
  await expect(second.locator(".ql-editor")).toHaveText("Page one writes");
  await expect(page.getByRole("status").filter({ hasText: "E2E chapter" })).toHaveText("E2E chapter · Saved");
  const firstSaved = await request.get(chapterUrl);
  expect(firstSaved.ok()).toBe(true);
  const firstHead = (await firstSaved.json()).head_rev;
  expect(firstHead).toBeGreaterThan(chapter.head_rev);

  await second.locator(".ql-editor").fill("Page one writes; page two replies");
  await expect(page.locator(".ql-editor")).toHaveText("Page one writes; page two replies");
  await expect(second.getByRole("status").filter({ hasText: "E2E chapter" })).toHaveText("E2E chapter · Saved");
  const savedResponse = await request.get(chapterUrl);
  expect(savedResponse.ok()).toBe(true);
  const saved = await savedResponse.json();
  expect(saved.head_rev).toBeGreaterThan(firstHead);
  expect(saved.content).toEqual({ ops: [{ insert: "Page one writes; page two replies\n" }] });

  for (const tab of [page, second]) {
    await tab.reload();
    await expect(tab.locator(".ql-editor")).toHaveText("Page one writes; page two replies");
    await expect(tab.getByRole("status").filter({ hasText: "E2E chapter" })).toHaveText("E2E chapter · Saved");
  }
  expect(failures).toEqual([]);
});
