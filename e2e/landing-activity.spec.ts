import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("recent activity opens an edited chapter after reload", async ({ page, request }, testInfo) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const post = async (url: string, data: object) => {
    const response = await request.post(`${api}${url}`, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const title = `Activity ${testInfo.project.name} ${testInfo.retry}`;
  const series = await post("/series", { title, position: 0 });
  const book = await post(`/series/${series.id}/books`, { title, position: 0 });
  const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, { title, position: 0, kind: "chapter" });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await editor.click();
  await page.keyboard.type("Recent activity edit");
  await page.getByRole("button", { name: "Library", exact: true }).click();
  await expect(page.getByRole("region", { name: "Resume", exact: true })).toBeVisible();
  await page.reload();
  const activity = page.getByRole("region", { name: "Recent activity" });
  await expect(activity.getByText(title, { exact: true })).toBeVisible();
  await activity.getByRole("button", { name: "Open", exact: true }).first().click();
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page.locator(".ql-editor")).toContainText("Recent activity edit");
});
