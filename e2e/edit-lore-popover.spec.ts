import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openChapter(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const seriesResponse = await request.post(`${api}/series`, { data: { title: "Popover series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const base = `${api}/series/${series.id}`;
  const bookResponse = await request.post(`${base}/books`, { data: { title: "Book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${base}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening Mira arrived.\n" }] },
  } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  expect((await request.post(`${base}/lore`, { data: {
    kind: "character", name: "Mira", visible_text: "The navigator.", level: "known_dont_reveal",
  } })).status()).toBe(201);
  await page.setViewportSize({ width, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(editor).toHaveText("Opening Mira arrived.");
}

async function openLore(page: Page) {
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const trigger = page.getByRole("region", { name: "Lore in window", exact: true }).getByRole("button", { name: /Mira/ });
  await trigger.click();
  await expect(page.getByRole("dialog", { name: "Lore match", exact: true })).toBeVisible();
  return trigger;
}

async function openEdit(page: Page) {
  const editor = page.locator(".ql-editor");
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
  const trigger = page.getByRole("button", { name: "Edit selection", exact: true });
  await expect(trigger).toBeEnabled();
  await trigger.click();
  await expect(page.getByRole("dialog", { name: "Edit instruction", exact: true })).toBeVisible();
  return trigger;
}

async function expectOutsideDismissal(page: Page, trigger: Locator, dialog: Locator) {
  await expect(trigger).toHaveAttribute("aria-expanded", "true");
  const id = await dialog.getAttribute("id");
  expect(id).toBeTruthy();
  await expect(trigger).toHaveAttribute("aria-controls", id!);
  await page.locator(".editor-chapter-title").click();
  await expect(dialog).toBeHidden();
  await expect(trigger).toHaveAttribute("aria-expanded", "false");
}

for (const width of [1100, 1440]) {
  test(`Lore match has no close button at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openLore(page);
    await expect(page.getByRole("button", { name: "Close lore match", exact: true })).toHaveCount(0);
  });

  test(`Escape dismisses Lore match at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openLore(page);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: "Lore match", exact: true })).toBeHidden();
  });

  test(`Lore match links its trigger and dismisses on outside click at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    const trigger = await openLore(page);
    await expectOutsideDismissal(page, trigger, page.getByRole("dialog", { name: "Lore match", exact: true }));
  });

  test(`Outside click dismisses Lore match at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openLore(page);
    await page.locator(".editor-chapter-title").click();
    await expect(page.getByRole("dialog", { name: "Lore match", exact: true })).toBeHidden();
  });

  test(`Edit preserves its note and selection on Escape at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openEdit(page);
    const dialog = page.getByRole("dialog", { name: "Edit instruction", exact: true });
    const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
    await expect(note).toBeFocused();
    await expect(note).toHaveCSS("background-color", "rgb(243, 214, 216)");
    await expect(note).toHaveCSS("border-width", "0px");
    await expect(note).toHaveCSS("border-radius", "1px");
    expect(await note.evaluate(element => getComputedStyle(element).boxShadow)).not.toBe("none");
    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.y).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(box!.y + box!.height).toBeLessThanOrEqual(900);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(page.locator(".ql-editor")).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
  });

  test(`Edit links its trigger and dismisses on outside click at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    const trigger = await openEdit(page);
    await expectOutsideDismissal(page, trigger, page.getByRole("dialog", { name: "Edit instruction", exact: true }));
  });
}

test("Left-out lore opens from text but stays out of the context window", async ({ page, request }) => {
  for (const width of [1100, 1440]) {
    await openChapter(page, request, width);
    const api = process.env.QUILLAI_E2E_API_URL;
    const seriesId = new URL(page.url()).searchParams.get("seriesId");
    expect((await request.post(`${api}/series/${seriesId}/lore`, { data: {
      kind: "character", name: "Opening", visible_text: "Not sent.", level: "leave_out",
    } })).status()).toBe(201);
    await page.reload();
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const list = page.getByRole("region", { name: "Lore in window", exact: true });
    await expect(list).toContainText("Mira");
    await expect(list).not.toContainText("Opening");
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const point = await editor.evaluate(element => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 7);
      const rect = range.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    });
    await page.mouse.move(point.x, point.y);
    const card = page.getByRole("dialog", { name: "Lore match", exact: true });
    await expect(card).toBeVisible();
    await expect(card).toContainText("Opening");
    await expect(card).toContainText("character · Leave out");
    await page.keyboard.press("Escape");
    await expect(card).toBeHidden();
    await page.mouse.move(0, 0);
    await editor.focus();
    await editor.evaluate(element => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 0);
      range.collapse(true);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
    });
    await page.keyboard.press("Alt+l");
    await expect(card).toBeVisible();
    await expect(card).toContainText("Opening");
    await expect(card).toContainText("character · Leave out");
    await page.keyboard.press("Escape");
    await expect(card).toBeHidden();
  }
});
