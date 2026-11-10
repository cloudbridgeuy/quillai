import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const FLAGGED_TEXT = "She walked very quickly home.";
const RULES_SHIPPED = false;

async function openFlaggedChapter(page: Page, request: APIRequestContext, width: number) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Hints", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Hints", position: 0 } })).json();
  const chapter = await (await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Hints", position: 0, kind: "chapter", content: { ops: [{ insert: `${FLAGGED_TEXT}\n` }] },
  } })).json();
  await page.setViewportSize({ width, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  return { apiUrl, seriesId: series.id as number, bookId: book.id as number, chapterId: chapter.id as number };
}

for (const width of [1100, 1440]) {
  test(`a flagged word is underlined and its tip opens and closes without changing the chapter at ${width}px`, async ({ page, request }) => {
    test.fixme(!RULES_SHIPPED, "RULES is empty until GUZ-692 ships a rule; set RULES_SHIPPED and FLAGGED_TEXT to the first rule's trigger");
    const { apiUrl, seriesId, bookId, chapterId } = await openFlaggedChapter(page, request, width);
    const chapterUrl = `${apiUrl}/series/${seriesId}/books/${bookId}/chapters/${chapterId}`;
    const before = await (await request.get(chapterUrl)).json();
    const underline = page.locator("[data-hint-underline]").first();
    await expect(underline).toBeVisible();
    const box = await underline.boundingBox();
    if (!box) throw new Error("underline has no box");
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    const tip = page.getByRole("tooltip");
    await expect(tip).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(tip).toBeHidden();
    expect(await (await request.get(chapterUrl)).json()).toEqual(before);
  });
}
