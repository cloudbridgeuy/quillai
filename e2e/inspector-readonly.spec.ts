import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openInspector(page: Page, request: APIRequestContext, width: number, edit = false) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Inspector read-only", position: 0 } })).json();
  const base = `${api}/series/${series.id}`;
  const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "A quiet harbour.\n" }] },
  } })).json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  if (edit) {
    await editor.click();
    await editor.evaluate(element => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 2);
      range.setEnd(text, 15);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
    });
  }
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const inspector = page.locator("#context-drawer");
  if (edit) {
    await inspector.getByText("Advanced AI controls", { exact: true }).click();
    await inspector.getByRole("combobox", { name: "Request to inspect" }).selectOption("edit");
  }
  return inspector;
}

for (const width of [1100, 1440]) {
  for (const edit of [false, true]) {
    test(`Document is read-only for ${edit ? "Edit" : "Continue"} at ${width}px`, async ({ page, request }) => {
      const inspector = await openInspector(page, request, width, edit);
      await expect(inspector.getByRole("textbox", { name: "Document prompt section", exact: true })).toHaveJSProperty("readOnly", true);
    });

    for (const section of ["Document", "Selection"]) {
      test(`no Reset ${section} for ${edit ? "Edit" : "Continue"} at ${width}px`, async ({ page, request }) => {
        const inspector = await openInspector(page, request, width, edit);
        await expect(inspector.getByRole("button", { name: `Reset ${section}`, exact: true })).toHaveCount(0);
      });
    }
  }

  test(`Selection is read-only for Edit at ${width}px`, async ({ page, request }) => {
    const inspector = await openInspector(page, request, width, true);
    await expect(inspector.getByRole("textbox", { name: "Selection prompt section", exact: true })).toHaveJSProperty("readOnly", true);
  });

  for (const section of ["Instruction", "Story context"]) {
    test(`${section} remains editable and appears in Full prompt at ${width}px`, async ({ page, request }) => {
      const inspector = await openInspector(page, request, width);
      const field = inspector.getByRole("textbox", { name: `${section} prompt section`, exact: true });
      await expect(field).toBeEditable();
      const text = `Changed ${section} for the next request.`;
      await field.fill(text);
      await inspector.getByRole("button", { name: "Full prompt", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Full prompt", exact: true })).toContainText(text);
    });
  }
}
