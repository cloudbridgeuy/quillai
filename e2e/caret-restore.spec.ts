import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";

type Sample = { scroll: number; text: string; top: number };

function observe(marker: string) {
  const state = window as unknown as { caretSamples: Sample[] };
  state.caretSamples = [];
  function frame() {
    const editor = document.querySelector(".ql-editor");
    if (editor?.textContent?.includes(marker)) {
      const selection = window.getSelection();
      const range = selection?.rangeCount ? selection.getRangeAt(0) : null;
      state.caretSamples.push({ scroll: window.scrollY, text: selection?.toString() ?? "", top: range?.getBoundingClientRect().top ?? -1 });
    }
    if (state.caretSamples.length < 6) requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

async function samples(page: Page): Promise<Sample[]> {
  await expect.poll(() => page.evaluate(() => (window as unknown as { caretSamples: Sample[] }).caretSamples?.length ?? 0)).toBe(6);
  return page.evaluate(() => (window as unknown as { caretSamples: Sample[] }).caretSamples);
}

for (const width of [1100, 1440]) {
  test(`restores chapter selection and viewport before the first frame at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: `Caret series ${width}`, position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: "Caret book", position: 0 });
    const url = `${api}/series/${series.id}/books/${book.id}/chapters`;
    const text = Array.from({ length: 100 }, (_, index) => `Chapter line ${index} with enough words for a long page.\n`).join("");
    const first = await post(url, { title: `Caret first ${width}`, position: 0, kind: "chapter", content: { ops: [{ insert: text }] } });
    await post(url, { title: `Caret second ${width}`, position: 1, kind: "chapter", content: { ops: [{ insert: "Second chapter\n" }] } });
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(first.id) })}`);
    const editor = page.locator(".ql-editor");
    await expect(editor).toContainText("Chapter line 90");
    const line = editor.locator("p").nth(70);
    await line.scrollIntoViewIfNeeded();
    await line.evaluate((element) => {
      const node = element.firstChild!;
      const range = document.createRange();
      range.setStart(node, 0);
      range.setEnd(node, 15);
      const selection = window.getSelection()!;
      selection.removeAllRanges();
      selection.addRange(range);
    });
    await expect.poll(() => page.evaluate(() => window.getSelection()?.toString())).toBe("Chapter line 70");
    await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("quillai.editor.position.v1") ?? "{}")[Object.keys(JSON.parse(localStorage.getItem("quillai.editor.position.v1") ?? "{}"))[0] ?? ""]?.length)).toBe(15);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(page.getByRole("button", { name: `Caret second ${width}`, exact: true })).toBeVisible();
    await line.scrollIntoViewIfNeeded();
    await expect.poll(() => page.evaluate(() => {
      const positions = JSON.parse(localStorage.getItem("quillai.editor.position.v1") ?? "{}");
      const offsetTop = positions[Object.keys(positions)[0] ?? ""]?.offsetTop;
      const selection = window.getSelection();
      return typeof offsetTop === "number" && selection?.rangeCount ? Math.abs(offsetTop - selection.getRangeAt(0).getBoundingClientRect().top) : Infinity;
    })).toBeLessThanOrEqual(1);
    const originalTop = await page.evaluate(() => window.getSelection()!.getRangeAt(0).getBoundingClientRect().top);
    await page.getByRole("button", { name: `Caret second ${width}`, exact: true }).click();
    await expect(editor).toHaveText("Second chapter");
    await page.evaluate(observe, "Chapter line 90");
    await page.getByRole("button", { name: `Caret first ${width}`, exact: true }).click();
    const switched = await samples(page);
    expect(switched[0]!.text).toBe("Chapter line 70");
    expect(Math.abs(switched[0]!.top - originalTop)).toBeLessThanOrEqual(2);
    expect(switched.every((sample) => Math.abs(sample.scroll - switched[0]!.scroll) <= 1)).toBe(true);
    await page.addInitScript(observe, "Chapter line 90");
    await page.reload();
    const reloaded = await samples(page);
    expect(reloaded[0]!.text).toBe("Chapter line 70");
    expect(Math.abs(reloaded[0]!.top - originalTop)).toBeLessThanOrEqual(2);
    expect(reloaded.every((sample) => Math.abs(sample.scroll - reloaded[0]!.scroll) <= 1)).toBe(true);
  });
}
