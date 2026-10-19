import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";

for (const interleaving of ["append", "replace", "pending"] as const) {
  test(`two pages preserve emojis during concurrent ${interleaving}`, async ({ context, page, request }, testInfo) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    if (!apiUrl) throw new Error("Run this reproduction with cargo xtask e2e");
    const seriesResponse = await request.post(`${apiUrl}/series`, {
      data: { title: "Emoji series", position: 0 },
    });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
      data: { title: "Emoji book", position: 0 },
    });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const initial = "Opening 😀🚀🌙";
    const chapterResponse = await request.post(chaptersUrl, {
      data: {
        title: "Emoji chapter", position: 0, kind: "chapter",
        content: { ops: [{ insert: `${initial}\n` }] },
      },
    });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    const chapterUrl = `${chaptersUrl}/${chapter.id}`;
    const query = new URLSearchParams({
      apiBaseUrl: apiUrl,
      seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
    });
    const second = await context.newPage();
    const tabs = [page, second];
    const events: unknown[] = [];
    const failures: string[] = [];
    const recordings: Promise<void>[] = [];
    let held = 0;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    try {
      for (const [index, tab] of tabs.entries()) {
        tab.on("pageerror", (error) => {
          failures.push(error.message);
          events.push({ tab: index, pageerror: error.stack });
        });
        tab.on("requestfailed", (req) => {
          events.push({ tab: index, url: req.url(), request: req.postData(), failure: req.failure() });
        });
        tab.on("response", (response) => {
          if (!response.url().startsWith(apiUrl)) return;
          recordings.push((async () => {
            const entry = {
              tab: index, method: response.request().method(), url: response.url(),
              request: response.request().postData(), status: response.status(),
              response: await response.text().catch(String),
            };
            events.push(entry);
            if (entry.status >= 400) failures.push(JSON.stringify(entry));
          })());
        });
        await tab.goto(`/?${query}`);
        await expect(tab.locator(".ql-editor")).toHaveText(initial);
        await expect(tab.getByRole("status").filter({ hasText: "Emoji chapter" })).toHaveText("Emoji chapter · Saved");
        await tab.route(`${chapterUrl}/deltas`, async (route) => {
          if (route.request().method() === "POST") {
            held++;
            events.push({ tab: index, heldRequest: route.request().postData() });
            await gate;
          }
          await route.continue();
        });
      }

      await Promise.all(tabs.map(async (tab, index) => {
        const editor = tab.locator(".ql-editor");
        await editor.evaluate((element, { index, interleaving }) => {
          (element as HTMLElement).focus();
          const text = element.firstChild!.firstChild!;
          const length = text.textContent!.length;
          const start = interleaving === "replace" ? index === 0 ? 8 : length - 2 : index === 0 ? length : 0;
          const range = document.createRange();
          range.setStart(text, start);
          range.setEnd(text, start + (interleaving === "replace" ? 2 : 0));
          window.getSelection()!.removeAllRanges();
          window.getSelection()!.addRange(range);
        }, { index, interleaving });
        await tab.keyboard.insertText(interleaving === "replace"
          ? index === 0 ? "😁" : "🌟"
          : index === 0 ? " A😀🦊" : "B🚀🐋 ");
      }));
      await expect.poll(() => held).toBe(2);
      if (interleaving === "pending") {
        await Promise.all(tabs.map(async (tab, index) => {
          await tab.locator(".ql-editor").evaluate((element, index) => {
            (element as HTMLElement).focus();
            const text = element.firstChild!.firstChild!;
            const range = document.createRange();
            range.setStart(text, index === 0 ? text.textContent!.length : 0);
            range.collapse(true);
            window.getSelection()!.removeAllRanges();
            window.getSelection()!.addRange(range);
          }, index);
          await tab.keyboard.insertText(index === 0 ? " C😁🌈" : "D🪐🐙 ");
        }));
      }
      release();

      const expectedParts = interleaving === "replace"
        ? ["Opening 😁🚀🌟"]
        : [initial, "A😀🦊", "B🚀🐋", ...(interleaving === "pending" ? ["C😁🌈", "D🪐🐙"] : [])];
      for (const tab of tabs) {
        await expect(tab.getByRole("status").filter({ hasText: "Emoji chapter" })).toHaveText("Emoji chapter · Saved");
        for (const part of expectedParts) await expect(tab.locator(".ql-editor")).toContainText(part);
      }
      const converged = await page.locator(".ql-editor").innerText();
      await expect(second.locator(".ql-editor")).toHaveText(converged);
      await Promise.all(tabs.map(async (tab, index) => {
        await tab.locator(".ql-editor").evaluate((element) => {
          (element as HTMLElement).focus();
          const range = document.createRange();
          range.selectNodeContents(element.firstChild!);
          range.collapse(false);
          window.getSelection()!.removeAllRanges();
          window.getSelection()!.addRange(range);
        });
        await tab.keyboard.insertText(index === 0 ? " E🦄🍀" : " F🐬🎉");
      }));
      for (const tab of tabs) {
        for (const part of [...expectedParts, "E🦄🍀", "F🐬🎉"]) {
          await expect(tab.locator(".ql-editor")).toContainText(part);
        }
        await expect(tab.getByRole("status").filter({ hasText: "Emoji chapter" })).toHaveText("Emoji chapter · Saved");
      }
      const finalText = await page.locator(".ql-editor").innerText();
      await expect(second.locator(".ql-editor")).toHaveText(finalText);
      const savedResponse = await request.get(chapterUrl);
      expect(savedResponse.ok()).toBe(true);
      const saved = await savedResponse.json();
      events.push({ saved });
      expect(saved.head_rev).toBeGreaterThanOrEqual(chapter.head_rev + 4);
      expect(saved.content).toEqual({ ops: [{ insert: `${finalText.trimEnd()}\n` }] });
      for (const tab of tabs) {
        await tab.reload();
        await expect(tab.locator(".ql-editor")).toHaveText(finalText);
        await expect(tab.getByRole("status").filter({ hasText: "Emoji chapter" })).toHaveText("Emoji chapter · Saved");
      }
      expect(failures).toEqual([]);
    } finally {
      release();
      await Promise.all(recordings);
      for (const [index, tab] of tabs.entries()) {
        events.push({ tab: index, text: await tab.locator(".ql-editor").innerText().catch(String), statuses: await tab.getByRole("status").allTextContents() });
      }
      const diagnostics = JSON.stringify({ interleaving, failures, events }, null, 2);
      console.log(diagnostics);
      await testInfo.attach("emoji-sync-network", { body: diagnostics, contentType: "application/json" });
    }
  });
}

for (const seed of [730, 731, 20261019, 42, 123456789, 0xdeadbeef]) {
  test(`seeded rapid emoji clipboard pastes seed=${seed}`, async ({ context, page, request }, testInfo) => {
    console.log(`emoji-paste seed=${seed} pastesPerTab=50`);
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    if (!apiUrl) throw new Error("Run this reproduction with cargo xtask e2e");
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Rapid emoji series", position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Rapid emoji book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
    const initial = "Opening 😀🚀🌙\n";
    const payload = "😀🎉👍🏽❤️👨‍👩‍👧";
    const chapterResponse = await request.post(chaptersUrl, {
      data: { title: "Rapid emoji chapter", position: 0, kind: "chapter", content: { ops: [{ insert: initial }] } },
    });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    const chapterUrl = `${chaptersUrl}/${chapter.id}`;
    const query = new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
    const tabs = [page, await context.newPage()];
    const events: unknown[] = [];
    const failures: string[] = [];
    const recordings: Promise<void>[] = [];
    try {
      await Promise.all(tabs.map(async (tab, index) => {
        tab.on("pageerror", (error) => {
          failures.push(error.message);
          events.push({ tab: index, pageerror: error.stack });
        });
        tab.on("console", (message) => {
          events.push({ tab: index, console: message.type(), text: message.text() });
          if (message.type() === "error") failures.push(message.text());
        });
        tab.on("request", (req) => {
          if (req.url().includes("/deltas")) events.push({ tab: index, sent: Date.now(), method: req.method(), url: req.url(), request: req.postData() });
        });
        tab.on("requestfailed", (req) => events.push({ tab: index, url: req.url(), request: req.postData(), failure: req.failure() }));
        tab.on("response", (response) => {
          if (!response.url().startsWith(apiUrl)) return;
          recordings.push((async () => {
            const entry = { tab: index, received: Date.now(), method: response.request().method(), url: response.url(), request: response.request().postData(), status: response.status(), response: await response.text().catch(String) };
            events.push(entry);
            if (entry.status >= 400) failures.push(JSON.stringify(entry));
          })());
        });
        await tab.route(`${chapterUrl}/deltas*`, async (route) => {
          await new Promise((resolve) => setTimeout(resolve, route.request().method() === "POST" ? 350 : 150));
          await route.continue();
        });
        await tab.goto(`/?${query}`);
        await expect(tab.getByRole("status").filter({ hasText: "Rapid emoji chapter" })).toHaveText("Rapid emoji chapter · Saved");
      }));
      await Promise.all(tabs.map(async (tab, index) => {
        const operations = await tab.locator(".ql-editor").evaluate(async (element, { seed, index, payload }) => {
          type Fiber = { ref?: { current?: Quill }; child?: Fiber; sibling?: Fiber };
          let quill: Quill | undefined;
          for (let ancestor: Element | null = element; ancestor && !quill; ancestor = ancestor.parentElement) {
            const key = Object.keys(ancestor).find((key) => key.startsWith("__reactFiber$"));
            if (!key) continue;
            const root = (ancestor as unknown as Record<string, Fiber>)[key];
            const stack = root ? [root] : [];
            for (let visited = 0; stack.length && visited < 500; visited++) {
              const fiber = stack.pop()!;
              const candidate = fiber.ref?.current;
              if (candidate?.root === element) {
                quill = candidate;
                break;
              }
              if (fiber.child) stack.push(fiber.child);
              if (fiber.sibling) stack.push(fiber.sibling);
            }
          }
          if (!quill?.clipboard?.dangerouslyPasteHTML) throw new Error("Existing Quill instance not found through mounted React ref");
          let random = (seed ^ Math.imul(index + 1, 0x9e3779b9)) >>> 0;
          const operations: { paste: number; cursor: number; length: number; at: number }[] = [];
          for (let paste = 0; paste < 50; paste++) {
            random = (Math.imul(random, 1664525) + 1013904223) >>> 0;
            const text = quill.getText();
            const boundaries = [0];
            for (const point of text.slice(0, -1)) boundaries.push(boundaries[boundaries.length - 1]! + point.length);
            const cursor = boundaries[random % boundaries.length]!;
            quill.setSelection(cursor, 0, "user");
            quill.clipboard.dangerouslyPasteHTML(cursor, payload, "user");
            operations.push({ paste, cursor, length: text.length, at: Date.now() });
            await new Promise((resolve) => setTimeout(resolve, 45));
          }
          return operations;
        }, { seed, index, payload });
        events.push({ tab: index, operations });
      }));
      for (const tab of tabs) await expect(tab.getByRole("status").filter({ hasText: "Rapid emoji chapter" })).toHaveText("Rapid emoji chapter · Saved");
      await expect.poll(async () => {
        const savedResponse = await request.get(chapterUrl);
        expect(savedResponse.status()).toBe(200);
        const saved = await savedResponse.json();
        const text = saved.content.ops.map((op: { insert: string }) => op.insert).join("");
        const expected = [initial, ...Array<string>(100).fill(payload)].join("");
        expect([...text].sort()).toEqual([...expected].sort());
        const texts = await Promise.all(tabs.map((tab) => tab.locator(".ql-editor").evaluate((element) => (element as HTMLElement).innerText)));
        return texts.every((value) => value.trimEnd() === text.trimEnd());
      }).toBe(true);
      expect(failures).toEqual([]);
    } finally {
      await Promise.all(recordings);
      for (const [index, tab] of tabs.entries()) events.push({ tab: index, text: await tab.locator(".ql-editor").innerText().catch(String), statuses: await tab.getByRole("status").allTextContents(), recovery: await tab.evaluate(() => sessionStorage.getItem("quillai.chapter.recovery.v1")).catch(String) });
      const saved = await request.get(chapterUrl);
      events.push({ savedStatus: saved.status(), saved: await saved.text() });
      const diagnostics = JSON.stringify({ seed, payload, pastesPerTab: 50, failures, events }, null, 2);
      console.log(diagnostics);
      await testInfo.attach(`emoji-paste-seed-${seed}`, { body: diagnostics, contentType: "application/json" });
    }
  });
}
