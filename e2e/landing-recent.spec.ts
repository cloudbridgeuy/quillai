import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("recent edited chapters can be forgotten across reloads", async ({ page, request }, testInfo) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const post = async (url: string, data: object) => {
    const response = await request.post(`${api}${url}`, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const prefix = `Recent ${testInfo.project.name}`;
  const series = await post("/series", { title: `${prefix} series`, position: 0 });
  const book = await post(`/series/${series.id}/books`, { title: `${prefix} book`, position: 0 });
  for (const [index, name] of ["First", "Second"].entries()) {
    const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, { title: `${prefix} ${name}`, position: index, kind: "chapter" });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await editor.click();
    await page.keyboard.type(`Edited ${name}`);
    await page.getByRole("button", { name: "Library", exact: true }).click();
    await expect(page.getByRole("region", { name: "Resume", exact: true })).toBeVisible();
  }
  const resume = page.getByRole("region", { name: "Resume", exact: true });
  await expect(resume.getByText(`${prefix} First`, { exact: true })).toBeVisible();
  await expect(resume.getByText(`${prefix} Second`, { exact: true })).toBeVisible();
  const first = resume.locator("div").filter({ has: page.getByText(`${prefix} First`, { exact: true }) }).filter({ has: page.getByRole("button", { name: "Forget", exact: true }) }).last();
  await first.getByRole("button", { name: "Forget", exact: true }).click();
  await page.reload();
  await expect(resume.getByText(`${prefix} Second`, { exact: true })).toBeVisible();
  await expect(resume.getByText(`${prefix} First`, { exact: true })).toHaveCount(0);
});
