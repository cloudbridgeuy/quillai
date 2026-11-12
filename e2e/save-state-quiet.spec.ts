import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

test("save state stays quiet while saves succeed and shows the pill on failure", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Quiet series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Quiet book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: { title: "Quiet chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  const deltaUrl = `${chaptersUrl}/${chapter.id}/deltas`;

  await page.setViewportSize({ width: 1100, height: 1000 });
  const query = new URLSearchParams({
    apiBaseUrl: apiUrl,
    seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
  });
  await page.goto(`/?${query}`);
  const editor = page.locator(".ql-editor");
  const saveRegion = page.getByRole("status", { name: "Chapter save status", exact: true });
  await expect(editor).toHaveText("Opening line");
  await expectEditorReady(editor);
  await expectSaved(page);
  await expect(saveRegion).toHaveCSS("position", "absolute");
  await expect(saveRegion).toHaveCSS("width", "1px");
  await expect(saveRegion).not.toContainText("Unsaved changes");

  const savedPost = page.waitForResponse((response) => (
    response.url() === deltaUrl && response.request().method() === "POST" && response.ok()
  ));
  await editor.click();
  await page.keyboard.press("End");
  await page.keyboard.type(" more");
  await savedPost;
  await expectSaved(page);
  await expect(saveRegion).toHaveCSS("position", "absolute");
  await expect(saveRegion).toHaveCSS("width", "1px");
  await expect(saveRegion).not.toContainText("Unsaved changes");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveCount(0);

  await page.route(deltaUrl, (route) => (
    route.request().method() === "POST"
      ? route.fulfill({ status: 503, json: { error: "Save unavailable", code: "UNAVAILABLE" } })
      : route.continue()
  ));
  await page.keyboard.type(" again");
  const pill = page.getByRole("alert", { name: "Chapter save status", exact: true });
  await expect(pill).toBeVisible();
  await expect(pill.locator("span").first()).toHaveText("Not saved");
  await expect(page.locator(".editor-shell")).toHaveAttribute("data-save-state", "failed");
});
