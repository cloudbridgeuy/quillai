import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("Edit note toggles and overhangs its card", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Edit note series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Edit note book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Edit note chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  const edit = page.getByRole("button", { name: "Edit selection", exact: true });
  const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  const card = page.getByRole("dialog", { name: "Edit instruction", exact: true });
  await expectEditorReady(editor);
  await expect(editor).toHaveText("Opening line");
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await editor.click();
    await editor.evaluate((element) => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 7);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
    });
    await expect(edit).toBeEnabled();
    await expect(edit).toHaveAttribute("aria-expanded", "false");
    await edit.click();
    await expect(edit).toHaveAttribute("aria-expanded", "true");
    await expect(note).toBeFocused();
    await expect(page.getByRole("button", { name: "Cancel edit", exact: true })).toHaveCount(0);
    const cardBox = await card.boundingBox();
    const noteBox = await note.boundingBox();
    expect(cardBox).not.toBeNull();
    expect(noteBox).not.toBeNull();
    expect(noteBox!.x).toBeLessThan(cardBox!.x);
    for (const side of ["top", "right", "bottom", "left"]) await expect(card).toHaveCSS(`border-${side}-width`, "0px");
    expect(await card.evaluate((element) => getComputedStyle(element).boxShadow)).not.toBe("none");
    const fontSize = await note.evaluate((element) => getComputedStyle(element).fontSize);
    await expect(card.locator(".editor-edit-flag")).toHaveCSS("font-size", fontSize);
    await expect(card.locator("#edit-instruction-hint")).toHaveCSS("font-size", fontSize);
    await expect(card.getByRole("button", { name: "Apply", exact: true })).toHaveCSS("font-size", fontSize);
    await edit.click();
    await expect(edit).toHaveAttribute("aria-expanded", "false");
    await expect(note).toBeHidden();
    await expect(editor).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
  }
});
