import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import { calculateDocumentWindow } from "../app/src/md_editor/window";
import type { AiGenerateRequest } from "../app/src/md_editor/ai_contract";

const paragraphs = Array.from({ length: 50 }, (_, index) => `${index + 1}. The harbour lay quiet under the evening sky. Mara watched the lanterns move across the water and waited for the letter that would change her journey. The tide carried no answer.`);
const text = `${paragraphs.join("\n")}\n`;

async function select(page: Page, paragraph: number) {
  await page.locator(".ql-editor").evaluate((editor, index) => {
    const node = editor.querySelectorAll("p")[index]!.firstChild!;
    (editor as HTMLElement).focus({ preventScroll: true });
    const range = document.createRange();
    range.setStart(node, 10);
    range.collapse(true);
    window.getSelection()!.removeAllRanges();
    window.getSelection()!.addRange(range);
  }, paragraph);
}

async function expectGeometry(page: Page, start: number, end: number) {
  const bracket = page.locator(".editor-continue-window");
  await expect(bracket).toHaveCount(1);
  await expect(bracket).toHaveAttribute("data-start", String(start));
  await expect(bracket).toHaveAttribute("data-end", String(end));
  const measured = await page.locator(".ql-editor").evaluate((editor, positions) => {
    const bounds = (position: number) => {
      const walker = document.createTreeWalker(editor, NodeFilter.SHOW_TEXT);
      let node = walker.nextNode();
      let remaining = position;
      while (node) {
        const length = node.textContent!.length;
        if (remaining <= length) {
          const range = document.createRange();
          range.setStart(node, remaining);
          range.collapse(true);
          return range.getBoundingClientRect();
        }
        remaining -= length + 1;
        node = walker.nextNode();
      }
      throw new Error("Position outside chapter");
    };
    const first = bounds(positions.start);
    const last = bounds(positions.end);
    const marker = document.querySelector(".editor-continue-window")!;
    const rect = marker.getBoundingClientRect();
    const style = getComputedStyle(marker);
    return { topError: Math.abs(rect.top - first.top), bottomError: Math.abs(rect.bottom - last.bottom), width: rect.width,
      left: style.left, stroke: style.borderLeftWidth, color: style.borderLeftColor, opacity: style.opacity };
  }, { start, end });
  expect(measured.topError).toBeLessThan(1);
  expect(measured.bottomError).toBeLessThan(1);
  expect(measured).toMatchObject({ width: 9, left: "-24px", stroke: "2px", color: "rgb(124, 155, 176)", opacity: "0.6" });
  await expect(page.locator("[data-start][data-end]")).toHaveCount(1);
  await expect(page.locator(".bracket.lav, #br-resume, .editor-resume-window")).toHaveCount(0);
}

async function documentText(page: Page) {
  return page.locator(".ql-editor").evaluate((editor) => `${Array.from(editor.querySelectorAll("p"), (paragraph) => paragraph.textContent).join("\n")}\n`);
}

async function send(page: Page, detail: unknown) {
  await page.evaluate((detail) => window.dispatchEvent(new CustomEvent("continue-window-event", { detail })), detail);
}

for (const width of [1100, 1440]) {
  test(`Continue window geometry, flag, history and persistence at ${width}px`, async ({ page, request }) => {
    await page.setViewportSize({ width, height: 1000 });
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const series = await (await request.post(`${api}/series`, { data: { title: "Window series", position: 0 } })).json();
    const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Window book", position: 0 } })).json();
    const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, { data: {
      title: "The harbour at dusk", position: 0, kind: "chapter", content: { ops: [{ insert: text }] },
    } })).json();
    const requests: AiGenerateRequest[] = [];
    await page.route("**/ai/generate", async (route) => {
      requests.push(route.request().postDataJSON());
      await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "delta", delta: { ops: [{ insert: "A lantern stirred." }] } })}\n${JSON.stringify({ type: "done", finishReason: "length" })}\n` });
    });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings", exact: true });
    await settings.getByRole("spinbutton", { name: "Window Size", exact: true }).fill("400");
    await page.keyboard.press("Escape");
    const visibleCaret = paragraphs.slice(0, 3).reduce((sum, paragraph) => sum + paragraph.length + 1, 0) + 10;
    await select(page, 3);
    const visibleWindow = calculateDocumentWindow({ documentLength: text.length - 1, selection: { index: visibleCaret, length: 0 }, windowSize: 400 });
    await expectGeometry(page, visibleWindow.start, visibleWindow.end);
    await page.locator(".editor-continue-window").evaluate((node) => window.scrollBy(0, node.getBoundingClientRect().top - 100));
    const visibleBounds = await page.locator(".editor-continue-window").boundingBox();
    expect(visibleBounds!.y).toBeGreaterThanOrEqual(0);
    expect(visibleBounds!.y + visibleBounds!.height).toBeLessThan(1000);
    const flag = page.getByRole("button", { name: /Continue window starts ↑/ });
    await expect(flag).toBeHidden();
    await page.screenshot({ path: `../target/guz-757/${test.info().project.name}/${width}-flag-hidden.png` });
    const typedText = "An author revises the harbour. ";
    await page.keyboard.type(typedText);
    const sourceText = await documentText(page);
    const typedWindow = calculateDocumentWindow({ documentLength: sourceText.length - 1, selection: { index: visibleCaret + typedText.length, length: 0 }, windowSize: 400 });
    expect(typedWindow.start).toBeGreaterThan(visibleWindow.start);
    await expectGeometry(page, typedWindow.start, typedWindow.end);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    const caret = paragraphs.slice(0, 10).reduce((sum, paragraph) => sum + paragraph.length + 1, 0) + typedText.length + 10;
    await select(page, 10);
    await page.locator(".ql-editor p").nth(10).scrollIntoViewIfNeeded();
    const documentWindow = calculateDocumentWindow({ documentLength: sourceText.length - 1, selection: { index: caret, length: 0 }, windowSize: 400 });
    expect(documentWindow.start).toBeGreaterThan(0);
    await expectGeometry(page, documentWindow.start, documentWindow.end);
    for (const [distance, label] of [[750, "just above"], [999, "just above"], [1000, "1 pages up"], [1250, "1 pages up"], [1750, "2 pages up"]] as const) {
      await page.locator(".editor-continue-window").evaluate((node, distance) => window.scrollTo(0, Math.ceil(window.scrollY + node.getBoundingClientRect().top + distance)), distance);
      await expect.poll(() => page.locator(".editor-continue-window").evaluate((node) => -node.getBoundingClientRect().top)).toBeGreaterThanOrEqual(distance);
      expect(await page.locator(".editor-continue-window").evaluate((node) => -node.getBoundingClientRect().top)).toBeLessThan(distance + 1);
      await expect(flag).toHaveText(`Continue windowstarts ↑ ${label}`);
    }
    await flag.click();
    await expect(flag).toBeHidden();
    await expect.poll(() => page.locator(".editor-continue-window").evaluate((node) => Math.abs(node.getBoundingClientRect().top - 24))).toBeLessThan(1);
    const scrollBefore = await page.evaluate(() => window.scrollY);
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Continue writing with thread", exact: true })).toBeVisible();
    expect(requests[0]!.document.text).toBe(sourceText.slice(documentWindow.start, documentWindow.end));
    expect(requests[0]!.selection?.range).toEqual({ index: caret - documentWindow.start, length: 0 });
    expect(documentWindow.end).toBeGreaterThan(caret);
    expect(Math.abs(await page.evaluate(() => window.scrollY) - scrollBefore)).toBeLessThanOrEqual(2);
    await expect(page.locator(".editor-continue-window")).toHaveCount(1);
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await settings.getByRole("checkbox", { name: "Whole document", exact: true }).check();
    await page.keyboard.press("Escape");
    await select(page, 30);
    await page.locator(".ql-editor p").nth(30).scrollIntoViewIfNeeded();
    await expect(flag).toBeVisible();
    await expect(flag).toHaveText(/Continue windowstarts ↑ [\d.]+ pages up/);
    await expect(page.locator(".editor-continue-window")).toHaveAttribute("data-start", "0");
    await expectGeometry(page, 0, (await documentText(page)).length - 1);
    await expect.poll(async () => {
      const pages = await page.locator(".editor-continue-window").evaluate((node) => Math.round(-node.getBoundingClientRect().top / window.innerHeight));
      return (await flag.textContent()) === `Continue windowstarts ↑ ${pages} pages up`;
    }).toBe(true);
    await page.screenshot({ path: `../target/guz-757/${test.info().project.name}/${width}-flag-visible.png` });
    await flag.click();
    await expect(flag).toBeHidden();
    await expect.poll(() => page.locator(".editor-continue-window").evaluate((node) => Math.abs(node.getBoundingClientRect().top - 24))).toBeLessThan(1);
    await select(page, 0);
    const idleEnd = (await documentText(page)).length - 1;
    const idleHeight = (await page.locator(".editor-continue-window").boundingBox())!.height;
    await page.keyboard.type(" Author writing moves the complete window. ".repeat(8));
    await expectGeometry(page, 0, (await documentText(page)).length - 1);
    expect((await documentText(page)).length - 1).toBeGreaterThan(idleEnd);
    expect((await page.locator(".editor-continue-window").boundingBox())!.height).toBeGreaterThan(idleHeight);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await page.evaluate(() => {
      const original = window.fetch.bind(window);
      window.fetch = async (input, init) => {
        if (!String(input).endsWith("/ai/generate")) return original(input, init);
        const encoder = new TextEncoder();
        return new Response(new ReadableStream({
          start(controller) {
            const receive = (event: Event) => {
              const detail = (event as CustomEvent).detail;
              controller.enqueue(encoder.encode(`${JSON.stringify(detail)}\n`));
              if (detail.type !== "delta") {
                window.removeEventListener("continue-window-event", receive);
                controller.close();
              }
            };
            window.addEventListener("continue-window-event", receive);
            init?.signal?.addEventListener("abort", () => {
              window.removeEventListener("continue-window-event", receive);
              controller.error(new DOMException("Stopped", "AbortError"));
            }, { once: true });
          },
        }), { headers: { "Content-Type": "application/x-ndjson" } });
      };
    });
    await page.getByRole("button", { name: /Continue writing/ }).click();
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
    const streamHeight = (await page.locator(".editor-continue-window").boundingBox())!.height;
    await send(page, { type: "delta", delta: { ops: [{ insert: "Streamed lanterns cross the harbour. ".repeat(8) }] } });
    await expect(page.locator(".ql-editor")).toContainText("Streamed lanterns");
    await expectGeometry(page, 0, (await documentText(page)).length - 1);
    expect((await page.locator(".editor-continue-window").boundingBox())!.height).toBeGreaterThan(streamHeight);
    await select(page, 0);
    const authorHeight = (await page.locator(".editor-continue-window").boundingBox())!.height;
    await page.keyboard.type(" Concurrent author writing follows the window. ".repeat(8));
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
    await expectGeometry(page, 0, (await documentText(page)).length - 1);
    expect((await page.locator(".editor-continue-window").boundingBox())!.height).toBeGreaterThan(authorHeight);
    await send(page, { type: "delta", delta: { ops: [{ insert: " More distant lanterns. ".repeat(8) }] } });
    await expect(page.locator(".ql-editor")).toContainText("More distant lanterns");
    await expectGeometry(page, 0, (await documentText(page)).length - 1);
    await send(page, { type: "done", finishReason: "length" });
    await expect(page.getByRole("button", { name: "Continue writing with thread", exact: true })).toBeVisible();
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await page.locator(".ql-editor p").nth(30).scrollIntoViewIfNeeded();
    await expect(flag).toBeVisible();
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await settings.getByRole("checkbox", { name: "Show Continue window", exact: true }).uncheck();
    await expect(page.locator(".editor-continue-window")).toHaveCount(0);
    await expect(flag).toBeHidden();
    await page.reload();
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await expect(page.locator(".editor-continue-window")).toHaveCount(0);
    await expect(flag).toBeHidden();
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(settings.getByRole("checkbox", { name: "Show Continue window", exact: true })).not.toBeChecked();
    await settings.getByRole("checkbox", { name: "Show Continue window", exact: true }).check();
    await expect(page.locator(".editor-continue-window")).toHaveCount(1);
    await page.setViewportSize({ width: 1099, height: 1000 });
    await expect(page.locator(".editor-continue-window")).toHaveCount(0);
    await expect(flag).toBeHidden();
  });
}
