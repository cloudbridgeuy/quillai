import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

for (const scenario of ["continue", "typing", "stop", "edit"] as const) {
  test(`${scenario} waits for unsaved chapter changes`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
    const identity = `${scenario === "edit" ? "GUZ865-commentary" : "GUZ807-Finish"}-${crypto.randomUUID()}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const series = await post(`${api}/series`, { title: scenario, position: 0 });
    const book = await post(`${api}/series/${series.id}/books`, { title: scenario, position: 0 });
    const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
    const chapter = await post(chapters, { title: scenario, position: 0, kind: "chapter", content: { ops: [{ insert: `Opening line ${identity}\n` }] } });
    const chapterUrl = `${chapters}/${chapter.id}`;
    await page.setViewportSize({ width: 1100, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await expectSaved(page);

    const releases: Array<() => void> = [];
    const arrivals: Array<() => void> = [];
    const held = [0, 1].map(() => new Promise<void>(resolve => arrivals.push(resolve)));
    const gates = [0, 1].map(() => new Promise<void>(resolve => releases.push(resolve)));
    let posts = 0;
    let releasePoll!: () => void;
    let pollArrived!: () => void;
    const pollGate = new Promise<void>(resolve => { releasePoll = resolve; });
    const pollHeld = new Promise<void>(resolve => { pollArrived = resolve; });
    let pollBlocked = false;
    await page.route(`${chapterUrl}/deltas`, async route => {
      if (route.request().method() !== "POST") return route.continue();
      const index = posts++;
      if (index < 2) {
        arrivals[index]();
        await gates[index];
      }
      await route.continue();
    });
    await page.route(`${chapterUrl}/deltas?since=*`, async route => {
      if (posts > 0 && !pollBlocked) {
        pollBlocked = true;
        pollArrived();
        await pollGate;
      }
      await route.continue();
    });
    const before = (await (await request.get(`${provider}/control`)).json()).length as number;
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model: scenario === "edit" ? "e2e-edit" : "e2e-continue" } })).status()).toBe(200);
    let generations = 0;
    let generatedText = "";
    await page.route(`${api}/ai/generate`, route => {
      generations++;
      generatedText = route.request().postDataJSON().document.text;
      return route.continue();
    });
    try {
      await editor.click();
      await page.keyboard.press("End");
      await page.keyboard.type(" before");
      await held[0];
      if (scenario === "edit") {
        await editor.evaluate(element => {
          const text = element.querySelector("p")?.firstChild;
          if (!text) throw new Error("Missing chapter text");
          const range = document.createRange();
          range.setStart(text, 0);
          range.setEnd(text, 7);
          window.getSelection()?.removeAllRanges();
          window.getSelection()?.addRange(range);
        });
        await page.getByRole("button", { name: "Edit selection", exact: true }).click();
        await page.getByRole("textbox", { name: "Edit instruction", exact: true }).fill("Revise");
        await page.getByRole("button", { name: "Apply", exact: true }).click();
      } else {
        await page.getByRole("button", { name: "Continue writing", exact: true }).click();
      }
      const stop = page.getByRole("button", { name: "Stop AI", exact: true });
      await expect(stop).toBeVisible();
      await expect(page.getByText("Saving…", { exact: true })).toBeVisible();
      expect(generations).toBe(0);
      if (scenario === "typing") {
        await editor.click();
        await page.keyboard.press("End");
        await page.keyboard.type(" during");
      }
      if (scenario === "stop") await stop.click();
      releases[0]();
      await pollHeld;
      expect(generations).toBe(0);
      releasePoll();
      if (scenario === "typing") {
        await held[1];
        await expect(page.getByText("Saving…", { exact: true })).toBeVisible();
        expect(generations).toBe(0);
        releases[1]();
      }
      if (scenario === "stop") {
        await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
        await expectSaved(page);
        expect(generations).toBe(0);
        expect((await (await request.get(`${provider}/control`)).json()).length).toBe(before);
        expect((await (await request.get(chapterUrl)).json()).content).toEqual({ ops: [{ insert: `Opening line ${identity} before\n` }] });
      } else {
        const recorded = await request.get(`${provider}/control/${before}/request`);
        expect(recorded.status()).toBe(200);
        expect((await recorded.json()).identity).toBe(identity);
        expect(generations).toBe(1);
        const savedText = `Opening line ${identity} before${scenario === "typing" ? " during" : ""}`;
        expect(generatedText).toBe(savedText);
        expect((await (await request.get(chapterUrl)).json()).content).toEqual({ ops: [{ insert: `${savedText}\n` }] });
        const control = `${provider}/control/${before}`;
        const text = scenario === "edit" ? "<replacement>Revised</replacement>" : "generated";
        expect((await request.post(`${control}/fragment`, { data: { text } })).status()).toBe(200);
        expect((await request.post(`${control}/finish`)).status()).toBe(200);
        await expect(stop).toBeHidden();
        await expect(editor).toContainText(scenario === "edit" ? "Revised" : "generated");
        if (scenario === "typing") await expect(editor).toContainText("before during");
        await expect(page.getByText("Wait for chapter saving and polling to finish before starting AI.", { exact: true })).toHaveCount(0);
        expect(generations).toBe(1);
        expect((await (await request.get(`${provider}/control`)).json()).length).toBe(before + 1);
      }
    } finally {
      releases.forEach(release => release());
      releasePoll();
    }
  });
}
