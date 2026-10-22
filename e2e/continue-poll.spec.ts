import { createServer, type ServerResponse } from "node:http";
import { once } from "node:events";
import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const outcome of ["Finish", "Stop", "Stop waiting", "Unsaved", "Typed while waiting"] as const) {
  test(`${outcome}: Continue reserves a clean chapter during polling`, async ({ context, page, request }) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    if (!apiUrl) throw new Error("Run with cargo xtask e2e");
    const failures: string[] = [];
    context.on("response", response => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) failures.push(`${response.status()} ${response.url()}`);
    });
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: `Poll ${outcome}`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const chapterResponse = await request.post(chaptersUrl, { data: {
      title: "Poll reservation", position: 0, kind: "chapter", content: { ops: [{ insert: "\n" }] },
    } });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    const chapterUrl = `${chaptersUrl}/${chapter.id}`;
    const query = new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
    const second = await context.newPage();
    await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
    await page.clock.pauseAt(new Date("2026-01-01T00:00:00Z"));
    for (const tab of [page, second]) {
      tab.on("pageerror", error => failures.push(error.message));
      await tab.setViewportSize({ width: 1100, height: 1000 });
      await tab.goto(`/?${query}`);
      await expectEditorReady(tab.locator(".ql-editor"));
      await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    }
    let releasePoll!: () => void;
    const pollRelease = new Promise<void>(resolve => { releasePoll = resolve; });
    let pollHeld!: () => void;
    const held = new Promise<void>(resolve => { pollHeld = resolve; });
    let polls = 0;
    await page.route(`${chapterUrl}/deltas?since=*`, async route => {
      polls++;
      const response = await route.fetch();
      if (polls === 1) {
        pollHeld();
        await pollRelease;
      }
      await route.fulfill({ response });
    });
    let generations = 0;
    let openStream!: (response: ServerResponse) => void;
    const streaming = new Promise<ServerResponse>(resolve => { openStream = resolve; });
    const server = createServer((incoming, response) => {
      incoming.resume();
      const cors = {
        "Access-Control-Allow-Origin": new URL(page.url()).origin,
        "Access-Control-Allow-Headers": "Content-Type",
        "Access-Control-Allow-Methods": "POST",
      };
      if (incoming.method === "OPTIONS") {
        response.writeHead(204, cors);
        response.end();
        return;
      }
      response.writeHead(200, { ...cors, "Content-Type": "application/x-ndjson" });
      response.flushHeaders();
      openStream(response);
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    const address = server.address();
    if (!address || typeof address === "string") throw new Error("Missing stream server address");
    await page.route(`${apiUrl}/ai/generate`, route => {
      generations++;
      return route.continue({ url: `http://127.0.0.1:${address.port}/ai/generate` });
    });
    try {
      await page.clock.runFor(1000);
      await held;
      const editor = page.locator(".ql-editor");
      if (outcome === "Unsaved") {
        await editor.click();
        await page.keyboard.type("Retained author text");
      }
      const continueButton = page.getByRole("button", { name: "Continue writing", exact: true });
      await expect(continueButton).toBeEnabled();
      await continueButton.click();
      const stop = page.getByRole("button", { name: "Stop AI", exact: true });
      if (outcome === "Unsaved") {
        await expect(page.getByText("Wait for chapter saving and polling to finish before starting AI.", { exact: true })).toBeVisible();
        await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Unsaved changes");
        await expect(stop).toBeHidden();
      } else {
        await expect(stop).toBeVisible();
      }
      await page.clock.runFor(2000);
      expect(generations).toBe(0);
      expect(polls).toBe(1);
      if (outcome === "Stop waiting") {
        await stop.click();
        await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
      }
      if (outcome === "Typed while waiting") {
        await editor.click();
        await page.keyboard.type("Retained author text");
      }
      releasePoll();
      if (outcome === "Typed while waiting") {
        await expect(page.getByText("Wait for chapter saving and polling to finish before starting AI.", { exact: true })).toBeVisible();
        await expect(stop).toBeHidden();
      }
      if (outcome === "Finish" || outcome === "Stop") {
        const stream = await streaming;
        stream.write(`${JSON.stringify({ type: "delta", delta: { ops: [{ insert: "Generated text" }] } })}\n`);
        await expect(editor).toContainText("Generated text");
        if (outcome === "Finish") stream.end(`${JSON.stringify({ type: "done", finishReason: "stop" })}\n`);
        else await stop.click();
        await expect(stop).toBeHidden();
        expect(generations).toBe(1);
      }
      await expect.poll(async () => {
        await page.clock.runFor(1000);
        return page.getByRole("status", { name: "Chapter save status", exact: true }).textContent();
      }).toBe("Saved");
      const expectedText = outcome === "Unsaved" || outcome === "Typed while waiting" ? "Retained author text\n" : outcome === "Finish" ? "\nGenerated text\n" : "\n";
      await expect.poll(async () => {
        await page.clock.runFor(1000);
        const response = await request.get(chapterUrl);
        expect(response.status()).toBe(200);
        return (await response.json()).content;
      }).toEqual({ ops: [{ insert: expectedText }] });
      for (const tab of [page, second]) {
        await expect.poll(() => tab.locator(".ql-editor").evaluate(element => Array.from(element.querySelectorAll("p"), paragraph => `${paragraph.textContent}\n`).join(""))).toBe(expectedText);
      }
      if (outcome !== "Finish" && outcome !== "Stop") expect(generations).toBe(0);
      expect(polls).toBeGreaterThan(1);
      expect(failures).toEqual([]);
    } finally {
      releasePoll();
      await new Promise<void>((resolve, reject) => {
        server.close(error => error ? reject(error) : resolve());
        server.closeAllConnections();
      });
    }
  });
}
