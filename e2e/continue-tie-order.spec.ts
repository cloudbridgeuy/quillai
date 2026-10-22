import { appendFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
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
  test(`Continue server-priority paragraph-start tie order on ${outcome}`, async ({ context, page, request }) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    const providerUrl = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!apiUrl || !providerUrl) throw new Error("Run with cargo xtask e2e and its default catalog");
    const directory = resolve(import.meta.dir, "../target/guz-806", test.info().project.name);
    mkdirSync(directory, { recursive: true });
    const identity = `GUZ807-${outcome}-${crypto.randomUUID()}`;
    const opening = `Opening paragraph ${identity}`;
    const record = (value: unknown) => appendFileSync(`${directory}/deltas.log`, `${JSON.stringify({ outcome, identity, value })}\n`);
    const failures: string[] = [];
    const observations: Promise<void>[] = [];
    const submissions: Array<{ tab: string; submitted: unknown; returned: unknown }> = [];
    context.on("response", response => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.request().method()} ${response.url()}`);
      }
      if (response.url().startsWith(apiUrl) && response.url().includes("/deltas")) {
        observations.push((async () => {
          const observed = { tab: response.frame().page() === page ? "desktop" : "remote", submitted: response.request().postDataJSON(), returned: await response.json() };
          record({ ...observed, method: response.request().method(), url: response.url(), status: response.status() });
          if (response.request().method() === "POST") submissions.push(observed);
        })());
      }
    });
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: `GUZ-806 ${outcome}`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Tie order", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const chapterResponse = await request.post(chaptersUrl, { data: {
      title: "Continue paragraph-start tie", position: 0, kind: "chapter",
      content: { ops: [{ insert: `${opening}\nLater paragraph\n` }] },
    } });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    record({ initial: chapter, openingNewlineOffset: opening.length, remoteInsertOffset: opening.length + 1 });
    const chapterUrl = `${chaptersUrl}/${chapter.id}`;
    const second = await context.newPage();
    const query = new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
    for (const tab of [page, second]) {
      tab.on("pageerror", error => failures.push(error.message));
      await tab.setViewportSize({ width: 1100, height: 1000 });
      await tab.goto(`/?${query}`);
      await expectEditorReady(tab.locator(".ql-editor"));
      await expect(tab.locator(".ql-editor p")).toHaveText([opening, "Later paragraph"]);
      await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    }
    const requestsResponse = await request.get(`${providerUrl}/control`);
    expect(requestsResponse.status()).toBe(200);
    const before = (await requestsResponse.json()).length;
    const editor = page.locator(".ql-editor");
    const stop = page.getByRole("button", { name: "Stop AI", exact: true });
    await placeCaret(editor, 0, 3);
    expect((await request.post(`${providerUrl}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
    const generationResponse = page.waitForResponse(response => response.url() === `${apiUrl}/ai/generate` && response.request().method() === "POST");
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    const recorded = await request.get(`${providerUrl}/control/${before}/request`);
    expect(recorded.status()).toBe(200);
    const upstream = await recorded.json();
    expect(upstream).toEqual({ id: before, identity, model: "e2e-continue", stream: true, closed: false, completed: false });
    const controlUrl = `${providerUrl}/control/${upstream.id}`;
    expect((await request.post(`${controlUrl}/fragment`, { data: { text: "AI first" } })).status()).toBe(200);
    expect((await generationResponse).status()).toBe(200);
    await expect(stop).toBeVisible();
    await expect(editor.locator("p")).toHaveText([opening, "AI first", "Later paragraph"]);
    const remoteSave = second.waitForResponse(response => response.url() === `${chapterUrl}/deltas` && response.request().method() === "POST");
    await placeCaret(second.locator(".ql-editor"), 1, 0);
    await second.keyboard.type("REMOTE ");
    expect((await remoteSave).status()).toBe(200);
    await expect(second.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await expect(stop).toBeVisible();
    const duringResponse = await request.get(chapterUrl);
    expect(duringResponse.status()).toBe(200);
    const during = await duringResponse.json();
    record({ during });
    expect(during.content).toEqual({ ops: [{ insert: `${opening}\nREMOTE Later paragraph\n` }] });
    expect((await request.post(`${controlUrl}/fragment`, { data: { text: " tail" } })).status()).toBe(200);
    await expect(editor.locator("p")).toHaveText([opening, "AI first tail", "Later paragraph"]);
    if (outcome === "Finish") {
      expect((await request.post(`${controlUrl}/finish`)).status()).toBe(200);
    } else {
      await stop.click();
      await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
    }
    const closed = await request.get(`${controlUrl}/closed`);
    expect(closed.status()).toBe(200);
    expect(await closed.json()).toEqual({ ...upstream, closed: true, completed: outcome === "Finish" });
    await expect(stop).toBeHidden();
    const paragraphs = outcome === "Finish" ? [opening, "REMOTE AI first tail", "Later paragraph"] : [opening, "REMOTE Later paragraph"];
    const expectedText = `${paragraphs.join("\n")}\n`;
    try {
      for (const tab of [page, second]) {
        await expect(tab.locator(".ql-editor p")).toHaveText(paragraphs);
        await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
      }
    } finally {
      for (const tab of [page, second]) {
        record({ tab: tab === page ? "desktop" : "remote", text: await tab.locator(".ql-editor").evaluate(element => Array.from(element.querySelectorAll("p"), paragraph => `${paragraph.textContent}\n`).join("")) });
      }
      const savedResponse = await request.get(chapterUrl);
      expect(savedResponse.status()).toBe(200);
      const saved = await savedResponse.json();
      record({ finalServer: saved });
      await Promise.all(observations);
      expect(saved.content).toEqual({ ops: [{ insert: expectedText }] });
    }
    const offset = opening.length + 1;
    expect(submissions).toEqual([
      { tab: "remote", submitted: { baseRev: 0, delta: { ops: [{ retain: offset }, { insert: "REMOTE " }] } }, returned: { rev: 1, delta: { ops: [{ retain: offset }, { insert: "REMOTE " }] } } },
      ...(outcome === "Finish" ? [{ tab: "desktop", submitted: { baseRev: 0, delta: { ops: [{ retain: offset }, { insert: "AI first tail\n" }] } }, returned: { rev: 2, delta: { ops: [{ retain: offset + 7 }, { insert: "AI first tail\n" }] } } }] : []),
    ]);
    const finalRequests = await request.get(`${providerUrl}/control`);
    expect(finalRequests.status()).toBe(200);
    expect((await finalRequests.json()).length).toBe(before + 1);
    expect(failures).toEqual([]);
  });
}
