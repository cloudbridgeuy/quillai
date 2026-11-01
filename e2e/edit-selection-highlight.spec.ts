import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openChapter(page: Page, request: APIRequestContext, width: number) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  const providerUrl = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!apiUrl || !providerUrl) throw new Error("Run with cargo xtask e2e and its default catalog");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Selection highlight", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Selection highlight", position: 0 } })).json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, { data: {
    title: "Selection highlight", position: 0, kind: "chapter",
    content: { ops: [{ insert: "Opening line\nLater paragraph\n" }] },
  } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(editor.locator("p")).toHaveText(["Opening line", "Later paragraph"]);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  return { editor, providerUrl, chapterUrl: `${chaptersUrl}/${chapter.id}` };
}

async function openEdit(page: Page) {
  const editor = page.locator(".ql-editor");
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
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Edit instruction", exact: true })).toBeFocused();
}

async function selectionHighlights(page: Page) {
  return page.evaluate(() => Array.from(CSS.highlights.get("quillai-edit-selection") ?? []).map(range => range.toString()));
}

for (const width of [1100, 1440]) {
  test(`Edit selection stays pink while the instruction is focused at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openEdit(page);
    await expect.poll(() => selectionHighlights(page)).toContain("Opening");
  });

  test(`Edit selection highlight follows dismissal at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
    const edit = page.getByRole("button", { name: "Edit selection", exact: true });
    for (const dismissal of ["Escape", "outside", "toggle"]) {
      await openEdit(page);
      await expect.poll(() => selectionHighlights(page)).not.toEqual([]);
      if (dismissal === "Escape") await note.press("Escape");
      else if (dismissal === "outside") await page.locator(".editor-chapter-title").click();
      else await edit.click();
      await expect(note).toBeHidden();
      await expect.poll(() => selectionHighlights(page)).toEqual([]);
    }
  });

  test(`Apply replaces only the selected text at ${width}px`, async ({ page, request }) => {
    const { editor, providerUrl, chapterUrl } = await openChapter(page, request, width);
    const identity = `GUZ985-edit-${crypto.randomUUID()}`;
    const before = (await (await request.get(`${providerUrl}/control`)).json()).length as number;
    expect((await request.post(`${providerUrl}/control/arm`, { data: { identity, model: "e2e-edit" } })).status()).toBe(200);
    await openEdit(page);
    await page.getByRole("textbox", { name: "Edit instruction", exact: true }).fill(identity);
    const generation = page.waitForResponse(response => response.url().endsWith("/ai/generate") && response.request().method() === "POST");
    await page.getByRole("button", { name: "Apply", exact: true }).click();
    const controlUrl = `${providerUrl}/control/${before}`;
    const recorded = await request.get(`${controlUrl}/request`);
    expect(recorded.status()).toBe(200);
    expect(await recorded.json()).toMatchObject({ identity, model: "e2e-edit", stream: true });
    expect((await request.post(`${controlUrl}/fragment`, { data: { text: "<replacement>Revised</replacement>" } })).status()).toBe(200);
    expect((await request.post(`${controlUrl}/finish`)).status()).toBe(200);
    expect((await generation).status()).toBe(200);
    await expect(editor.locator("p")).toHaveText(["Revised line", "Later paragraph"]);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    expect((await (await request.get(chapterUrl)).json()).content).toEqual({ ops: [{ insert: "Revised line\nLater paragraph\n" }] });
    await expect.poll(() => selectionHighlights(page)).toEqual([]);
  });

  test(`GUZ-919 instruction text supports pointer and keyboard selection at ${width}px`, async ({ page, request }) => {
    await openChapter(page, request, width);
    await openEdit(page);
    const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
    await note.fill("Make this sentence vivid");
    const point = await note.evaluate(element => {
      const style = getComputedStyle(element);
      const canvas = document.createElement("canvas");
      const context = canvas.getContext("2d");
      if (!context) throw new Error("Missing canvas context");
      context.font = style.font;
      const rect = element.getBoundingClientRect();
      return { x: rect.x + parseFloat(style.paddingLeft), y: rect.y + parseFloat(style.paddingTop) + parseFloat(style.lineHeight) / 2, word: context.measureText("Make ").width, end: context.measureText("Make this").width };
    });
    const selected = () => note.evaluate((element: HTMLTextAreaElement) => element.value.slice(element.selectionStart, element.selectionEnd));
    await page.mouse.move(point.x + 1, point.y);
    await page.mouse.down();
    await page.mouse.move(point.x + point.end, point.y, { steps: 8 });
    await page.mouse.up();
    expect((await selected()).length).toBeGreaterThan(0);
    await page.mouse.dblclick(point.x + point.word + 4, point.y);
    expect(await selected()).toBe("this");
    await note.press("ArrowRight");
    await note.press("Shift+ArrowLeft");
    expect(await selected()).toBe("s");
  });
}
