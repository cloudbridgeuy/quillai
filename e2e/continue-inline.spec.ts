import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

function snapshot(element: Element, index?: number) {
  const host = element.closest(".editor-host")!;
  const key = Object.keys(host).find(key => key.startsWith("__reactFiber$"))!;
  type Fiber = { ref?: { current?: Quill }; return?: Fiber };
  let fiber: Fiber | undefined = (host as unknown as Record<string, Fiber>)[key];
  while (fiber) {
    const quill = fiber.ref?.current;
    if (quill?.root === element) {
      if (index !== undefined) quill.setSelection(index, 0, "user");
      return quill.getText();
    }
    fiber = fiber.return;
  }
  throw new Error("Quill instance not found");
}

test.beforeEach(async ({ page }) => {
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
            if (detail.type === "done") {
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
});

for (const scenario of [
  { name: "unfinished sentence", original: "Mara walked\n", caret: 11, expected: "Mara walked slowly\n" },
  { name: "finished sentence at paragraph end", original: "Mara walked.\n", caret: 12, expected: "Mara walked.\nslowly\n" },
  { name: "later text after caret", original: "Mara walked home.\n", caret: 11, expected: "Mara walked slowly home.\n" },
]) {
  test(`Continue places generated text at ${scenario.name}`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: scenario.name, position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: scenario.name, position: 0 });
    const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
      title: scenario.name, position: 0, kind: "chapter", content: { ops: [{ insert: scenario.original }] },
    });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await expectSaved(page);
    expect(await editor.evaluate(snapshot, scenario.caret)).toBe(scenario.original);
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("ai-test-event", { detail: { type: "delta", delta: { ops: [{ insert: "slowly" }] } } })));
    await expect.poll(() => editor.evaluate(snapshot)).toBe(scenario.expected);
    const tinted = await page.evaluate(() => Array.from(CSS.highlights.get("quillai-stream") ?? []).map(range => range.toString()).join(""));
    expect(tinted).toBe(scenario.name === "finished sentence at paragraph end" ? "slowly" : " slowly");
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("ai-test-event", { detail: { type: "done", finishReason: "stop" } })));
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
    expect(await editor.evaluate(snapshot)).toBe(scenario.expected);
  });
}

for (const scenario of [
  { name: "inserted space", original: "Mara walked home.\n", caret: 11, fragment: "slowly", expected: "Mara walked slowly home.\n", tint: " slowly" },
  { name: "midword text", original: "Mara\n", caret: 2, fragment: "x", expected: "Maxra\n", tint: "x" },
]) test(`Stop removes inline AI ${scenario.name}`, async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Stop inline", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Stop inline", position: 0 });
  const original = scenario.original;
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Stop inline", position: 0, kind: "chapter", content: { ops: [{ insert: original }] },
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expectSaved(page);
  expect(await editor.evaluate(snapshot, scenario.caret)).toBe(original);
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
  await page.evaluate(fragment => window.dispatchEvent(new CustomEvent("ai-test-event", { detail: { type: "delta", delta: { ops: [{ insert: fragment }] } } })), scenario.fragment);
  await expect.poll(() => editor.evaluate(snapshot)).toBe(scenario.expected);
  expect(await page.evaluate(() => Array.from(CSS.highlights.get("quillai-stream") ?? []).map(range => range.toString()).join(""))).toBe(scenario.tint);
  await page.getByRole("button", { name: "Stop AI", exact: true }).click();
  await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
  expect(await editor.evaluate(snapshot)).toBe(original);
});
