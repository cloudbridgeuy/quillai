import { createServer, type ServerResponse } from "node:http";
import { once } from "node:events";
import { expect, test, type Locator } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function placeCaret(editor: Locator, paragraph: number, offset: number) {
  await expectEditorReady(editor);
  await editor.evaluate((element, position) => {
    (element as HTMLElement).focus();
    const text = element.querySelectorAll("p")[position.paragraph]?.firstChild;
    if (!text) throw new Error("Missing paragraph text");
    const range = document.createRange();
    range.setStart(text, position.offset);
    range.collapse(true);
    const selection = window.getSelection()!;
    selection.removeAllRanges();
    selection.addRange(range);
  }, { paragraph, offset });
}

for (const outcome of ["Finish", "Stop"] as const) {
  test(`${outcome} converges two tabs and server after remote edits during Continue`, async ({ context, page, request }) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    if (!apiUrl) throw new Error("Run with cargo xtask e2e");
    const failures: string[] = [];
    context.on("response", response => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.request().method()} ${response.url()}`);
      }
    });
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: `${outcome} stream series`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Stream book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const chapterResponse = await request.post(chaptersUrl, { data: {
      title: "Streaming with remote edits", position: 0, kind: "chapter",
      content: { ops: [{ insert: "Opening paragraph\nLater paragraph\n" }] },
    } });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    const chapterUrl = `${chaptersUrl}/${chapter.id}`;
    const second = await context.newPage();
    const query = new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
    for (const tab of [page, second]) {
      tab.on("pageerror", error => failures.push(error.message));
      await tab.setViewportSize({ width: 1100, height: 1000 });
      await tab.goto(`/?${query}`);
      await expectEditorReady(tab.locator(".ql-editor"));
      await expect(tab.locator(".ql-editor p")).toHaveText(["Opening paragraph", "Later paragraph"]);
      await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    }

    let openStream!: (response: ServerResponse) => void;
    const streaming = new Promise<ServerResponse>(resolve => { openStream = resolve; });
    const server = createServer((incoming, response) => {
      incoming.resume();
      response.writeHead(200, {
        "Content-Type": "application/x-ndjson",
        "Access-Control-Allow-Origin": new URL(page.url()).origin,
      });
      response.flushHeaders();
      openStream(response);
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    const address = server.address();
    if (!address || typeof address === "string") throw new Error("Missing stream server address");
    await page.route(`${apiUrl}/ai/generate`, route => route.continue({ url: `http://127.0.0.1:${address.port}/ai/generate` }));

    try {
      const editor = page.locator(".ql-editor");
      const remoteEditor = second.locator(".ql-editor");
      const stop = page.getByRole("button", { name: "Stop AI", exact: true });
      await placeCaret(editor, 0, 3);
      await page.getByRole("button", { name: "Continue writing", exact: true }).click();
      const stream = await streaming;
      await expect(stop).toBeVisible();
      stream.write(`${JSON.stringify({ type: "delta", delta: { ops: [{ insert: "AI first" }] } })}\n`);
      await expect(editor.locator("p").nth(1)).toHaveText("AI first");
      await placeCaret(editor, 0, 3);
      await page.keyboard.type("AUTHOR");
      await expect(editor.locator("p").first()).toHaveText("OpeAUTHORning paragraph");

      const remoteSave = second.waitForResponse(response => response.url() === `${chapterUrl}/deltas` && response.request().method() === "POST");
      await placeCaret(remoteEditor, 1, 6);
      await second.keyboard.type("REMOTE ");
      expect((await remoteSave).status()).toBe(200);
      await expect(second.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
      await expect(stop).toBeVisible();
      expect(stream.writableEnded).toBe(false);
      const duringResponse = await request.get(chapterUrl);
      expect(duringResponse.status()).toBe(200);
      expect((await duringResponse.json()).content).toEqual({ ops: [{ insert: "Opening paragraph\nLater REMOTE paragraph\n" }] });
      await expect(editor.locator("p")).toHaveText(["OpeAUTHORning paragraph", "AI first", "Later paragraph"]);
      expect(failures).toEqual([]);

      if (outcome === "Finish") {
        stream.write(`${JSON.stringify({ type: "delta", delta: { ops: [{ insert: " tail" }] } })}\n`);
        await expect(editor.locator("p").nth(1)).toHaveText("AI first tail");
        stream.end(`${JSON.stringify({ type: "done", finishReason: "stop" })}\n`);
      } else {
        await stop.click();
        await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
      }
      await expect(stop).toBeHidden();
      const paragraphs = ["OpeAUTHORning paragraph", ...(outcome === "Finish" ? ["AI first tail"] : []), "Later REMOTE paragraph"];
      const expectedText = `${paragraphs.join("\n")}\n`;
      for (const tab of [page, second]) {
        await expect(tab.locator(".ql-editor p")).toHaveText(paragraphs);
        await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
        expect(await tab.locator(".ql-editor").evaluate(element => Array.from(element.querySelectorAll("p"), paragraph => `${paragraph.textContent}\n`).join(""))).toBe(expectedText);
      }
      const savedResponse = await request.get(chapterUrl);
      expect(savedResponse.status()).toBe(200);
      expect((await savedResponse.json()).content).toEqual({ ops: [{ insert: expectedText }] });
      expect(failures).toEqual([]);
    } finally {
      await new Promise<void>((resolve, reject) => {
        server.close(error => error ? reject(error) : resolve());
        server.closeAllConnections();
      });
    }
  });
}
