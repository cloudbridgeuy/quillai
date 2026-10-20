import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("desktop Continue preserves concurrent typing, selection and exact rollback", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.addInitScript(() => {
    const original = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      if (!String(input).endsWith("/ai/generate")) return original(input, init);
      const encoder = new TextEncoder();
      return new Response(new ReadableStream({
        start(controller) {
          const send = (event: Event) => {
            const detail = (event as CustomEvent).detail;
            controller.enqueue(encoder.encode(`${JSON.stringify(detail)}\n`));
            if (detail.type !== "delta") {
              window.removeEventListener("ai-test-event", send);
              controller.close();
            }
          };
          window.addEventListener("ai-test-event", send);
          init?.signal?.addEventListener("abort", () => {
            window.removeEventListener("ai-test-event", send);
            controller.error(new DOMException("Stopped", "AbortError"));
          }, { once: true });
        },
      }), { headers: { "Content-Type": "application/x-ndjson" } });
    };
  });
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Stream series", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Stream book", position: 0 } })).json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await (await request.post(chaptersUrl, { data: {
    title: "Concurrent writing", position: 0, kind: "chapter",
    content: { ops: [{ insert: "Opening paragraph\nLater paragraph\n" }] },
  } })).json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expect(editor).toContainText("Opening paragraph");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  const select = async (paragraph: number, start: number, end = start) => {
    await editor.evaluate((element, args) => {
      const walker = document.createTreeWalker(element.querySelectorAll("p")[args.paragraph]!, NodeFilter.SHOW_TEXT);
      const text = walker.nextNode();
      if (!text) throw new Error("No text");
      const range = document.createRange();
      range.setStart(text, args.start);
      range.setEnd(text, args.end);
      const selection = window.getSelection()!;
      selection.removeAllRanges();
      selection.addRange(range);
      (element as HTMLElement).focus();
    }, { paragraph, start, end });
  };
  const send = async (detail: unknown) => page.evaluate((detail) => {
    window.dispatchEvent(new CustomEvent("ai-test-event", { detail }));
  }, detail);
  const fragment = async (text: string) => send({ type: "delta", delta: { ops: [{ insert: text }] } });
  await select(0, 3);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await expect(editor).toBeEditable();
  await page.keyboard.type("AUTHOR");
  await fragment("AI first");
  await expect(editor.locator("p").nth(1)).toHaveText("AI first");
  expect(await page.evaluate(() => window.getSelection()?.anchorOffset)).toBe(9);
  await select(2, 2, 7);
  await fragment(" second");
  await expect(editor.locator("p").nth(1)).toHaveText("AI first second");
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("ter p");
  await select(1, 3);
  await page.keyboard.type("KEPT");
  await fragment(" tail");
  await expect(editor.locator("p").nth(1)).toHaveText("AI KEPTfirst second tail");
  expect(await page.evaluate(() => window.getSelection()?.anchorOffset)).toBe(7);
  const tintText = await page.evaluate(() => Array.from(CSS.highlights.get("quillai-stream") ?? []).map((range) => range.toString()).join(""));
  expect(tintText).not.toContain("KEPT");
  expect(tintText).toContain("AI first second tail");
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-756/during-${width}.png`, fullPage: true });
  }
  await page.getByRole("button", { name: "Stop AI", exact: true }).click();
  await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
  await expect(editor).toHaveText("OpeAUTHORning paragraphKEPTLater paragraph");
  expect(await page.evaluate(() => {
    const selection = window.getSelection();
    return selection?.anchorNode?.textContent?.slice(0, selection.anchorOffset);
  })).toBe("OpeAUTHORning paragraphKEPT");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await select(0, 1);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await fragment("Failed AI");
  await expect(editor).toContainText("Failed AI");
  await select(1, 6);
  await page.keyboard.type("SURVIVES");
  await send({ type: "error", error: "Mock stream failure" });
  await expect(page.getByText("Mock stream failure", { exact: true })).toBeVisible();
  await expect(editor).toHaveText("OpeAUTHORning paragraphKEPTSURVIVESLater paragraph");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await select(0, 2, 5);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await fragment("Completed AI");
  await expect(editor).toContainText("Completed AI");
  await send({ type: "done", finishReason: "stop" });
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("eAU");
  await expect(editor).toHaveAttribute("data-ai-tint", "fading");
  await expect(editor).not.toHaveAttribute("data-ai-tint", "fading");
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-756/after-${width}.png`, fullPage: true });
  }
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByText("Advanced AI controls", { exact: true }).click();
  await page.getByRole("checkbox", { name: "Tint AI text", exact: true }).uncheck();
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.reload();
  await expect(editor).toHaveText("OpeAUTHORning paragraphKEPTSURVIVESCompleted AILater paragraph");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByText("Advanced AI controls", { exact: true }).click();
  await expect(page.getByRole("checkbox", { name: "Tint AI text", exact: true })).not.toBeChecked();
  await page.getByRole("button", { name: "Close context", exact: true }).click();
  await select(0, 1);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await fragment("Server cancelled AI");
  await expect(editor).toContainText("Server cancelled AI");
  expect(await page.evaluate(() => CSS.highlights.get("quillai-stream")?.size)).toBe(0);
  await send({ type: "cancelled" });
  await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
  await expect(editor).toHaveText("OpeAUTHORning paragraphKEPTSURVIVESCompleted AILater paragraph");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.setViewportSize({ width: 1099, height: 1000 });
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await expect(editor).toHaveAttribute("contenteditable", "false");
  await page.setViewportSize({ width: 1100, height: 1000 });
  await expect(editor).toHaveAttribute("contenteditable", "false");
  await page.getByRole("button", { name: "Stop AI", exact: true }).click();
  await expect(editor).toBeEditable();
});
