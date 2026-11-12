import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

async function openChapter(page: Page, request: APIRequestContext) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Desktop edges", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Edge book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Edge chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening paragraph\nLater paragraph\n" }] },
  } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.addInitScript(() => {
    const original = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      if (!String(input).endsWith("/ai/generate")) return original(input, init);
      const encoder = new TextEncoder();
      return new Response(new ReadableStream({
        start(controller) {
          const receive = (event: Event) => {
            const detail = (event as CustomEvent).detail;
            if (detail.type === "disconnect") {
              controller.error(new TypeError("Stream connection lost"));
            } else if (detail.type === "eof") {
              controller.close();
            } else {
              controller.enqueue(encoder.encode(`${JSON.stringify(detail)}\n`));
              if (detail.type === "delta") return;
              controller.close();
            }
            window.removeEventListener("desktop-edge-event", receive);
          };
          window.addEventListener("desktop-edge-event", receive);
          init?.signal?.addEventListener("abort", () => {
            window.removeEventListener("desktop-edge-event", receive);
            controller.error(new DOMException("Stopped", "AbortError"));
          }, { once: true });
        },
      }), { headers: { "Content-Type": "application/x-ndjson" } });
    };
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expect(page.locator(".ql-editor")).toHaveText("Opening paragraphLater paragraph");
  await expectSaved(page);
}

async function select(page: Page, paragraph: number, start: number, end = start) {
  await page.locator(".ql-editor").evaluate((element, args) => {
    (element as HTMLElement).focus();
    const text = element.querySelectorAll("p")[args.paragraph]?.firstChild;
    if (!text) throw new Error("Missing paragraph text");
    const range = document.createRange();
    range.setStart(text, args.start);
    range.setEnd(text, args.end);
    const selection = window.getSelection()!;
    selection.removeAllRanges();
    selection.addRange(range);
  }, { paragraph, start, end });
}

async function send(page: Page, detail: unknown) {
  await page.evaluate((detail) => window.dispatchEvent(new CustomEvent("desktop-edge-event", { detail })), detail);
}

for (const width of [1100, 1440]) {
  test(`Manuscript rail toggles active state and restores selection at ${width}px`, async ({ page, request }) => {
    await page.setViewportSize({ width, height: 1000 });
    await openChapter(page, request);
    const editor = page.locator(".ql-editor");
    const manuscript = page.getByRole("button", { name: "Manuscript", exact: true });
    const drawer = page.locator("#manuscript-drawer");
    const rail = page.locator(".editor-rail-left");
    await expectEditorReady(editor);
    await select(page, 0, 0, 7);
    await expect(manuscript).toHaveAttribute("aria-controls", "manuscript-drawer");
    await expect(manuscript).toHaveAttribute("aria-expanded", "false");
    await manuscript.click();
    await expect(drawer).toBeVisible();
    await expect(page.getByRole("button", { name: "Close manuscript", exact: true })).toHaveCount(0);
    await expect(manuscript).toHaveAttribute("aria-expanded", "true");
    await expect(editor).toBeFocused();
    await page.mouse.move(500, 300);
    await expect(rail).toHaveCSS("opacity", "0.95");
    await expect(manuscript).toHaveCSS("color", "rgb(31, 41, 51)");
    await expect(manuscript).toHaveCSS("background-color", "rgb(215, 230, 208)");
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
    await page.screenshot({ path: `../target/guz-798/${test.info().project.name}/manuscript-open-${width}.png` });
    await editor.evaluate(element => (element as HTMLElement).blur());
    await manuscript.click();
    await expect(drawer).toBeHidden();
    await expect(manuscript).toHaveAttribute("aria-expanded", "false");
    await expect(editor).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
    await page.mouse.move(500, 300);
    await expect(rail).toHaveCSS("opacity", "0.35");
    await expect(manuscript).toHaveCSS("color", "rgb(111, 117, 124)");
    await expect(manuscript).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await page.screenshot({ path: `../target/guz-798/${test.info().project.name}/manuscript-closed-${width}.png` });
    for (const modifier of ["Control", "Meta"]) {
      await page.keyboard.press(`${modifier}+[`);
      await expect(drawer).toBeVisible();
      await expect(manuscript).toHaveAttribute("aria-expanded", "true");
      await page.keyboard.press(`${modifier}+[`);
      await expect(drawer).toBeHidden();
      await expect(manuscript).toHaveAttribute("aria-expanded", "false");
      await expect(editor).toBeFocused();
      expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
    }
    await manuscript.click();
    await expect(drawer).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(drawer).toBeHidden();
    await expect(manuscript).toHaveAttribute("aria-expanded", "false");
    await expect(editor).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
  });

  for (const dismissal of ["Escape", "outside click"]) {
    test(`Settings ${dismissal} preserves an active Continue at ${width}px`, async ({ page, request }) => {
      await page.setViewportSize({ width, height: 1000 });
      await openChapter(page, request);
      const editor = page.locator(".ql-editor");
      const stop = page.getByRole("button", { name: "Stop AI", exact: true });
      const settings = page.getByRole("dialog", { name: "Settings", exact: true });
      await select(page, 0, 3);
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      await expect(stop).toBeVisible();
      await send(page, { type: "delta", delta: { ops: [{ insert: "AI first" }] } });
      await expect(editor.locator("p").nth(0)).toHaveText("OpeAI firstning paragraph");
      await select(page, 1, 0, 5);
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      await expect(settings).toBeVisible();
      await expect(settings.getByRole("spinbutton", { name: "Window Size", exact: true })).toBeDisabled();
      await expect(settings.getByRole("checkbox", { name: "Whole document", exact: true })).toBeDisabled();
      const tint = settings.getByRole("checkbox", { name: "Tint AI text", exact: true });
      await expect(tint).toBeEnabled();
      await tint.uncheck();
      expect(await page.evaluate(() => CSS.highlights.get("quillai-stream")?.size)).toBe(0);
      await send(page, { type: "delta", delta: { ops: [{ insert: " second" }] } });
      await expect(editor.locator("p").nth(0)).toHaveText("OpeAI first secondning paragraph");
      await tint.check();
      expect(await page.evaluate(() => Array.from(CSS.highlights.get("quillai-stream") ?? []).map((range) => range.toString()).join(""))).toContain("AI first second");
      if (dismissal === "Escape") {
        await page.keyboard.press("Escape");
        await expect(editor).toBeFocused();
        expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Later");
      } else {
        await page.locator(".editor-chapter-title").click();
      }
      await expect(settings).toBeHidden();
      await expect(stop).toBeVisible();
      await expectEditorReady(editor);
      await select(page, 1, 5);
      await page.keyboard.type(" AUTHOR");
      await send(page, { type: "delta", delta: { ops: [{ insert: " tail" }] } });
      await expect(editor).toHaveText("OpeAI first second tailning paragraphLater AUTHOR paragraph");
      await send(page, { type: "done", finishReason: "stop" });
      await expect(stop).toBeHidden();
      await expectSaved(page);
      await page.reload();
      await expect(editor).toHaveText("OpeAI first second tailning paragraphLater AUTHOR paragraph");
    });
  }

  for (const failure of [
    { type: "disconnect", message: "Stream connection lost" },
    { type: "eof", message: "AI stream ended before a terminal event arrived." },
  ]) {
    test(`Continue ${failure.type} retains author typing inside AI text at ${width}px`, async ({ page, request }) => {
      await page.setViewportSize({ width, height: 1000 });
      await openChapter(page, request);
      const editor = page.locator(".ql-editor");
      await select(page, 0, 3);
      await page.getByRole("button", { name: "Continue", exact: true }).click();
      const stop = page.getByRole("button", { name: "Stop AI", exact: true });
      await expect(stop).toBeVisible();
      await send(page, { type: "delta", delta: { ops: [{ insert: "Generated words" }] } });
      await expect(editor.locator("p").nth(0)).toHaveText("OpeGenerated wordsning paragraph");
      await select(page, 0, 7);
      await page.keyboard.type("KEPT");
      await expect(editor.locator("p").nth(0)).toHaveText("OpeGeneKEPTrated wordsning paragraph");
      await send(page, { type: failure.type });
      await expect(page.getByText(failure.message, { exact: true })).toBeVisible();
      await expect(stop).toBeHidden();
      await expectEditorReady(editor);
      await expect(editor).toHaveText("OpeKEPTning paragraphLater paragraph");
      expect(await page.evaluate(() => CSS.highlights.get("quillai-stream")?.size)).toBe(0);
      await page.keyboard.type(" MORE");
      await expect(editor).toHaveText("OpeKEPT MOREning paragraphLater paragraph");
      await expectSaved(page);
      await page.reload();
      await expect(editor).toHaveText("OpeKEPT MOREning paragraphLater paragraph");
    });
  }
}
