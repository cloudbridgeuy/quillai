import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };

const api = process.env.QUILLAI_E2E_API_URL;
const opsText = (ops: Op[]) => ops.map(op => typeof op.insert === "string" ? op.insert : "").join("");

async function post(request: APIRequestContext, url: string, data: object, status = 201) {
  const response = await request.post(url, { data });
  expect(response.status(), await response.text()).toBe(status);
  return response.json();
}

async function openChapter(page: Page, request: APIRequestContext, title: string, content = "\n") {
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await post(request, `${api}/series`, { title, position: 0 });
  const book = await post(request, `${api}/series/${series.id}/books`, { title, position: 0 });
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await post(request, chapters, { title, position: 0, kind: "chapter", content: { ops: [{ insert: content }] } });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expectSaved(page);
  await editor.click();
  await page.keyboard.press("ControlOrMeta+End");
  const savedOps = async (): Promise<Op[]> => (await (await request.get(`${chapters}/${chapter.id}`)).json()).content.ops;
  return { editor, saved: async () => opsText(await savedOps()), savedOps };
}

for (const width of [1100, 1440]) {
  test.describe(`dialogue typing at ${width}px`, () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
    });

    test("typed double quotes save curly and survive reload", async ({ page, request }) => {
      const { editor, saved } = await openChapter(page, request, `Quotes ${width}`);
      await page.keyboard.type("\"Hi,\" she said.");
      await expect.poll(saved).toBe("“Hi,” she said.\n");
      await page.reload();
      await expect(editor).toHaveText("“Hi,” she said.");
    });

    test("apostrophes, nested quotes, and the '90s fix-up", async ({ page, request }) => {
      const { saved } = await openChapter(page, request, `Apostrophes ${width}`);
      await page.keyboard.type("\"I don't say 'no'\" in the '90s");
      await expect.poll(saved).toBe("“I don’t say ‘no’” in the ’90s\n");
    });

    test("double hyphens become em dashes mid-line", async ({ page, request }) => {
      const { saved } = await openChapter(page, request, `Dashes ${width}`);
      await page.keyboard.type("--Hola --dijo--.");
      await expect.poll(saved).toBe("—Hola —dijo—.\n");
    });

    test("one undo restores the straight quote and a second removes it", async ({ page, request }) => {
      const { editor } = await openChapter(page, request, `Undo ${width}`);
      await page.keyboard.type("\"");
      await expect(editor).toHaveText("“");
      await page.keyboard.press("ControlOrMeta+z");
      await expect(editor).toHaveText("\"");
      await page.keyboard.press("ControlOrMeta+z");
      await expect(editor).toHaveText("");
    });

    test("pasted quotes and dashes stay straight", async ({ page, request }) => {
      const { editor, saved } = await openChapter(page, request, `Paste ${width}`);
      await editor.evaluate((element) => {
        const data = new DataTransfer();
        data.setData("text/plain", "\"x\" -- 'y'");
        element.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
      });
      await expect.poll(saved).toBe("\"x\" -- 'y'\n");
    });

    test("markdown bold still formats and a quote after it closes", async ({ page, request }) => {
      const { saved, savedOps } = await openChapter(page, request, `Markdown ${width}`);
      await page.keyboard.type("**bold** \"x\"");
      await expect.poll(saved).toBe("bold “x”\n");
      expect((await savedOps()).find(op => op.insert === "bold")?.attributes).toEqual({ bold: true });
    });

    test("composition commits of one quote convert", async ({ page, request, browserName }) => {
      test.skip(browserName !== "chromium", "CDP IME only exists in Chromium");
      const { editor, saved } = await openChapter(page, request, `Composition ${width}`);
      const session = await page.context().newCDPSession(page);
      await page.keyboard.type("a ");
      await session.send("Input.imeSetComposition", { text: "\"", selectionStart: 1, selectionEnd: 1 });
      await expect(editor).toHaveText("a \"");
      await session.send("Input.insertText", { text: "\"" });
      await expect.poll(saved).toBe("a “\n");
    });

    test("Continue stream text stays straight, typed quotes convert, and -- stays during the stream", async ({ page, request }) => {
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
            },
          }), { headers: { "Content-Type": "application/x-ndjson" } });
        };
      });
      const { editor, saved } = await openChapter(page, request, `Stream ${width}`, "Mara walked\n");
      await page.getByRole("button", { name: "Continue writing", exact: true }).click();
      await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeVisible();
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("ai-test-event", { detail: { type: "delta", delta: { ops: [{ insert: " and said \"Go\"" }] } } })));
      await expect(editor).toContainText("and said \"Go\"");
      await page.keyboard.press("ControlOrMeta+End");
      await page.keyboard.type(" \"--");
      await expect(editor).toContainText("“--");
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("ai-test-event", { detail: { type: "done", finishReason: "stop" } })));
      await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
      await expect.poll(saved).toContain("and said \"Go\"");
      expect(await saved()).toContain("“--");
    });

    test("a screenplay group keeps typed quotes and hyphens", async ({ page, request }) => {
      if (!api) throw new Error("Run with cargo xtask e2e");
      const universe = await post(request, `${api}/universes`, { title: `Script ${width}`, position: 0 });
      const group = await post(request, `${api}/universes/${universe.id}/pieces`, { title: "Hall script", position: 0, label: "screenplay", shape: "group" });
      const scene = await post(request, `${api}/pieces/${group.id}/children`, { title: "Hall", position: 0, label: "scene" });
      await post(request, `${api}/pieces/${scene.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Rain falls.\n" }] } }, 200);
      await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
      const editor = page.locator(".ql-editor");
      await expectEditorReady(editor);
      await expect(editor).toContainText("Rain falls.");
      await editor.click();
      await page.keyboard.press("ControlOrMeta+End");
      await page.keyboard.type(" \"Hi\" --");
      await expect.poll(async () => opsText((await (await request.get(`${api}/pieces/${scene.id}`)).json()).content.ops)).toContain("Rain falls. \"Hi\" --");
    });
  });
}
