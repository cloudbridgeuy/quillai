import { expect, test, type Locator } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

async function editorState(editor: Locator, selection?: { index: number; length: number }) {
  return editor.evaluate((element, selection) => {
    type Fiber = { ref?: { current?: Quill }; child?: Fiber; sibling?: Fiber };
    let quill: Quill | undefined;
    for (let ancestor: Element | null = element; ancestor && !quill; ancestor = ancestor.parentElement) {
      const key = Object.keys(ancestor).find((key) => key.startsWith("__reactFiber$"));
      if (!key) continue;
      const root = (ancestor as unknown as Record<string, Fiber>)[key];
      const stack = root ? [root] : [];
      for (let visited = 0; stack.length && visited < 500; visited++) {
        const fiber = stack.pop()!;
        if (fiber.ref?.current?.root === element) {
          quill = fiber.ref.current;
          break;
        }
        if (fiber.child) stack.push(fiber.child);
        if (fiber.sibling) stack.push(fiber.sibling);
      }
    }
    if (!quill) throw new Error("Mounted Quill instance not found");
    if (selection) quill.setSelection(selection.index, selection.length, "user");
    const range = quill.getSelection();
    const text = quill.getText();
    const insideSurrogate = (index: number) => (
      text.charCodeAt(index - 1) >= 0xd800 && text.charCodeAt(index - 1) <= 0xdbff &&
      text.charCodeAt(index) >= 0xdc00 && text.charCodeAt(index) <= 0xdfff
    );
    return {
      text, content: quill.getContents(), selection: range,
      splitEndpoints: range ? [range.index, range.index + range.length].filter(insideSurrogate) : [],
    };
  }, selection);
}

test("remote edits keep emoji caret and selection boundaries safe", async ({ context, page, request }, testInfo) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const failures: string[] = [];
  const events: unknown[] = [];
  const tabs = [page, await context.newPage()];
  for (const [index, tab] of tabs.entries()) {
    tab.on("pageerror", (error) => failures.push(error.message));
    tab.on("response", (response) => {
      if (!response.url().startsWith(apiUrl)) return;
      const entry = { tab: index, status: response.status(), url: response.url(), request: response.request().postData() };
      events.push(entry);
      if (entry.status >= 400) failures.push(JSON.stringify(entry));
    });
    await tab.setViewportSize({ width: 1100, height: 1000 });
  }
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Emoji cursor series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Emoji cursor book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const cases = [
    { name: "a-before", index: 5, length: 0, inserted: "B ", insertAt: 0 },
    { name: "b-after", index: 7, length: 0, inserted: "B ", insertAt: 0 },
    { name: "c-spanning", index: 4, length: 3, inserted: "B ", insertAt: 0 },
    { name: "e-shared-high-surrogate", index: 7, length: 0, inserted: "😃", insertAt: 5 },
    { name: "d-inside", index: 6, length: 0, inserted: "B ", insertAt: 0 },
  ];
  try {
    for (const scenario of cases) {
      await test.step(scenario.name, async () => {
        const initial = "Lead 😀😃 tail\n";
        const chapterResponse = await request.post(chaptersUrl, {
          data: { title: scenario.name, position: 0, kind: "chapter", content: { ops: [{ insert: initial }] } },
        });
        expect(chapterResponse.status()).toBe(201);
        const chapter = await chapterResponse.json();
        const chapterUrl = `${chaptersUrl}/${chapter.id}`;
        const query = new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
        for (const tab of tabs) {
          await tab.goto(`/?${query}`);
          await expectEditorReady(tab.locator(".ql-editor"));
          await expect(tab.locator(".ql-editor")).toHaveText(initial);
          await expectSaved(tab);
        }
        const editor = page.locator(".ql-editor");
        const secondEditor = tabs[1]!.locator(".ql-editor");
        const selected = await editorState(editor, { index: scenario.index, length: scenario.length });
        events.push({ case: scenario.name, stage: "setSelection", requested: { index: scenario.index, length: scenario.length }, ...selected });
        console.log(JSON.stringify({ case: scenario.name, requested: scenario.index, actual: selected.selection, splitEndpoints: selected.splitEndpoints }));
        expect(selected.selection).not.toBeNull();
        if (scenario.name !== "d-inside") {
          expect(selected.selection).toEqual({ index: scenario.index, length: scenario.length });
          expect(selected.splitEndpoints).toEqual([]);
        }

        let remoteText = initial;
        for (const operation of ["insert", "delete"] as const) {
          await editorState(secondEditor, { index: scenario.insertAt, length: operation === "delete" ? scenario.inserted.length : 0 });
          const committed = tabs[1]!.waitForResponse((response) => response.url() === `${chapterUrl}/deltas` && response.request().method() === "POST");
          if (operation === "insert") {
            await tabs[1]!.keyboard.insertText(scenario.inserted);
            remoteText = initial.slice(0, scenario.insertAt) + scenario.inserted + initial.slice(scenario.insertAt);
          } else {
            await tabs[1]!.keyboard.press("Backspace");
            remoteText = initial;
          }
          expect((await committed).status()).toBe(200);
          await expect(editor).toHaveText(remoteText);
          const state = await editorState(editor);
          events.push({ case: scenario.name, stage: operation, ...state });
          expect(state.text).toBe(remoteText);
          expect(state.selection, `${scenario.name} after remote ${operation}`).not.toBeNull();
          expect(state.splitEndpoints, `${scenario.name} after remote ${operation}: ${JSON.stringify(state)}`).toEqual([]);
          if (scenario.name === "d-inside") expect(state.selection).toEqual({ index: remoteText.indexOf("😀"), length: 0 });
          expect(failures).toEqual([]);
        }

        if (scenario.length) await page.keyboard.press("ArrowRight");
        const beforeTyping = await editorState(editor);
        expect(beforeTyping.selection?.length).toBe(0);
        expect(beforeTyping.splitEndpoints).toEqual([]);
        const index = beforeTyping.selection!.index;
        const expected = initial.slice(0, index) + "x" + initial.slice(index);
        const committed = page.waitForResponse((response) => response.url() === `${chapterUrl}/deltas` && response.request().method() === "POST");
        await page.keyboard.insertText("x");
        expect((await committed).status()).toBe(200);
        for (const tab of tabs) {
          await expect(tab.locator(".ql-editor")).toHaveText(expected);
          await expectSaved(tab);
          const state = await editorState(tab.locator(".ql-editor"));
          expect(state.text).toBe(expected);
          expect(state.content).toEqual({ ops: [{ insert: expected }] });
          expect([...state.text].filter((point) => point.length === 2)).toEqual(["😀", "😃"]);
          expect(state.splitEndpoints).toEqual([]);
        }
        const savedResponse = await request.get(chapterUrl);
        expect(savedResponse.status()).toBe(200);
        const saved = await savedResponse.json();
        expect(saved.content).toEqual({ ops: [{ insert: expected }] });
        expect(saved.head_rev).toBe(chapter.head_rev + 3);
        expect(failures).toEqual([]);
        events.push({ case: scenario.name, result: "passed", saved });
        console.log(`${scenario.name}: passed; both tabs and server exactly match; emojis intact`);
      });
    }
  } finally {
    const diagnostics = JSON.stringify({ failures, events }, null, 2);
    console.log(diagnostics);
    await testInfo.attach("emoji-cursor-remote", { body: diagnostics, contentType: "application/json" });
  }
});
