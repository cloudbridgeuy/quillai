import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const friendly = "AI isn't set up yet. Add your AI provider details to quillai.env and restart QuillAI.";
const secret = "AI provider configuration error: missing required environment variable QUILLAI_AI_API_KEY";

test("AI setup errors are friendly across the editor", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "AI setup", position: 0 } })).json();
  const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  })).json();
  for (const path of ["models", "generate"]) {
    await page.route(`${api}/ai/${path}`, route => route.fulfill({ status: 503, json: { error: secret, code: "AI_NOT_CONFIGURED" } }));
  }
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.locator("#context-drawer").getByText("Advanced AI controls", { exact: true }).click();
  await expect(page.locator("#context-drawer").getByText(friendly, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await editor.click();
  await page.getByRole("button", { name: /^Continue writing/ }).click();
  await expect(page.locator(".editor-run-status")).toHaveText(friendly);
  await editor.click();
  await editor.evaluate(element => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 7);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  await note.fill("Rewrite");
  await note.press("Enter");
  await expect(page.getByRole("alert").filter({ hasText: friendly })).toBeVisible();
  await expect(page.locator("body")).not.toContainText(secret);
  await expect(page.locator("body")).not.toContainText("environment variable");
  await expect(page.locator(`[title*="environment variable"]`)).toHaveCount(0);
  await expect(page.locator(`[title*="QUILLAI_AI_API_KEY"]`)).toHaveCount(0);
});
