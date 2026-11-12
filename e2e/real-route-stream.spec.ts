import { expect, test, type Locator } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

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
  test(`${outcome} through real generation converges tabs and server`, async ({ context, page, request }) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    const providerUrl = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!apiUrl || !providerUrl) throw new Error("Run with cargo xtask e2e and its default catalog");
    const identity = `GUZ807-${outcome}-${crypto.randomUUID()}`;
    const opening = `Opening paragraph ${identity}`;
    const authored = `OpeAUTHORning paragraph ${identity}`;
    const failures: string[] = [];
    context.on("response", response => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.request().method()} ${response.url()}`);
      }
    });
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: `${outcome} real stream series`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Real stream book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const chapterResponse = await request.post(chaptersUrl, { data: {
      title: "Real streaming with remote edits", position: 0, kind: "chapter",
      content: { ops: [{ insert: `${opening}\nLater paragraph\n` }] },
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
      await expect(tab.locator(".ql-editor p")).toHaveText([opening, "Later paragraph"]);
      await expectSaved(tab);
    }
    async function upstreamRequests() {
      const response = await request.get(`${providerUrl}/control`);
      expect(response.status()).toBe(200);
      return response.json() as Promise<Array<{ id: number; model: string; stream: boolean; closed: boolean; completed: boolean }>>;
    }
    const before = (await upstreamRequests()).length;
    const editor = page.locator(".ql-editor");
    const remoteEditor = second.locator(".ql-editor");
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
    await expect(editor.locator("p").nth(0)).toHaveText(`OpeAI firstning paragraph ${identity}`);
    await placeCaret(editor, 0, 3);
    await page.keyboard.type("AUTHOR");
    await expect(editor.locator("p").first()).toHaveText(`OpeAUTHORAI firstning paragraph ${identity}`);

    const remoteSave = second.waitForResponse(response => response.url() === `${chapterUrl}/deltas` && response.request().method() === "POST");
    await placeCaret(remoteEditor, 1, 6);
    await second.keyboard.type("REMOTE ");
    expect((await remoteSave).status()).toBe(200);
    await expectSaved(second);
    await expect(stop).toBeVisible();
    expect((await upstreamRequests())[before]).toEqual(upstream);
    const duringResponse = await request.get(chapterUrl);
    expect(duringResponse.status()).toBe(200);
    expect((await duringResponse.json()).content).toEqual({ ops: [{ insert: `${opening}\nLater REMOTE paragraph\n` }] });
    await expect(editor.locator("p")).toHaveText([`OpeAUTHORAI firstning paragraph ${identity}`, "Later paragraph"]);
    expect(failures).toEqual([]);

    if (outcome === "Finish") {
      expect((await request.post(`${controlUrl}/fragment`, { data: { text: " tail" } })).status()).toBe(200);
      await expect(editor.locator("p").nth(0)).toHaveText(`OpeAUTHORAI first tailning paragraph ${identity}`);
      expect((await request.post(`${controlUrl}/finish`)).status()).toBe(200);
    } else {
      await stop.click();
      await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
    }
    const closed = await request.get(`${controlUrl}/closed`);
    expect(closed.status()).toBe(200);
    expect((await closed.json()).closed).toBe(true);
    expect((await upstreamRequests())[before]!.completed).toBe(outcome === "Finish");
    await expect(stop).toBeHidden();
    const paragraphs = [outcome === "Finish" ? `OpeAUTHORAI first tailning paragraph ${identity}` : authored, "Later REMOTE paragraph"];
    const expectedText = `${paragraphs.join("\n")}\n`;
    for (const tab of [page, second]) {
      await expect(tab.locator(".ql-editor p")).toHaveText(paragraphs);
      await expectSaved(tab);
      expect(await tab.locator(".ql-editor").evaluate(element => Array.from(element.querySelectorAll("p"), paragraph => `${paragraph.textContent}\n`).join(""))).toBe(expectedText);
    }
    const savedResponse = await request.get(chapterUrl);
    expect(savedResponse.status()).toBe(200);
    expect((await savedResponse.json()).content).toEqual({ ops: [{ insert: expectedText }] });
    expect((await upstreamRequests()).length).toBe(before + 1);
    expect(failures).toEqual([]);
  });
}
