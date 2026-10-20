import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";

async function continueControlTest({ page, request }: { page: Page; request: APIRequestContext }) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  await page.addInitScript(() => {
    const originalFetch = window.fetch.bind(window);
    const proof = { requests: [] as Array<{ messages: Array<{ role: string; content: string }> }>, aborts: 0 };
    Object.assign(window, { continueProof: proof });
    window.fetch = async (input, init) => {
      if (!String(input).endsWith("/ai/generate")) return originalFetch(input, init);
      proof.requests.push(JSON.parse(String(init?.body)));
      const encoder = new TextEncoder();
      const stream = new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(encoder.encode(`${JSON.stringify({ type: "delta", delta: { ops: [{ insert: proof.requests.length === 1 ? "A saved beginning." : " More words." }] } })}\n`));
          if (proof.requests.length === 1) {
            controller.enqueue(encoder.encode(`${JSON.stringify({ type: "done", finishReason: "length" })}\n`));
            controller.close();
          } else {
            init?.signal?.addEventListener("abort", () => {
              proof.aborts++;
              controller.error(new DOMException("Stopped", "AbortError"));
            }, { once: true });
          }
        },
      });
      return new Response(stream, { headers: { "Content-Type": "application/x-ndjson" } });
    };
  });
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Continue series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Continue book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Continue chapter", position: 0, kind: "chapter" },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await expect(editor).toBeEditable();
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  const continueButton = page.locator(".editor-continue");
  await expect(page.getByRole("button", { name: /^Continue/ })).toHaveCount(1);
  await expect(continueButton).toHaveAccessibleName("Continue writing");
  await expect(continueButton.locator(".editor-thread-indicator")).toHaveCount(0);
  await expect(page.locator(".editor-controls-panel")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Preview Context", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Edit", exact: true })).toHaveCount(0);
  await expect(page.locator(".editor-page").getByRole("heading", { name: "Continue chapter", exact: true })).toBeVisible();
  await expect(page.locator(".editor-page .ql-toolbar")).toBeVisible();
  await expect(page.locator(".editor-page textarea, .editor-page input:not(.ql-tooltip input)")).toHaveCount(0);
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-755/idle-${width}.png`, fullPage: true });
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(page.locator("#manuscript-drawer").getByRole("link", { name: "Export local chapter", exact: true })).toBeVisible();
    await page.screenshot({ path: `../target/guz-755/manuscript-${width}.png`, fullPage: true });
    await page.getByRole("button", { name: "Close manuscript", exact: true }).click();
  }
  await continueButton.click();
  await expect(editor).toHaveText("A saved beginning.");
  await expect(continueButton).toHaveAccessibleName("Continue writing with thread");
  await expect(continueButton.getByText("with thread", { exact: true })).toBeVisible();
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const context = page.locator("#context-drawer");
  await expect(context.getByRole("textbox", { name: "Story Context", exact: true })).toBeVisible();
  await expect(context.getByRole("textbox", { name: "Instruction", exact: true })).toBeVisible();
  await expect(context.getByRole("spinbutton", { name: "Window Size", exact: true })).toHaveValue("1200");
  await expect(context.getByRole("checkbox", { name: "Whole document", exact: true })).not.toBeChecked();
  await expect(context.getByRole("button", { name: "Reset Thread", exact: true })).toBeEnabled();
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-755/with-thread-${width}.png`, fullPage: true });
  }
  await expect(page.getByRole("option", { name: "Continue · with thread", exact: true })).toHaveCount(1);
  await page.getByRole("button", { name: "Close context", exact: true }).click();
  await continueButton.click();
  await expect(continueButton).toHaveAccessibleName("Stop AI");
  await expect(continueButton.getByText("with thread", { exact: true })).toBeVisible();
  await expect(editor).toContainText("More words.");
  await continueButton.click();
  await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
  await expect(editor).toBeEditable();
  await expect(editor).toHaveText("A saved beginning. More words.");
  await expect(continueButton).toHaveAccessibleName("Continue writing");
  await expect(continueButton.locator(".editor-thread-indicator")).toHaveCount(0);
  const proof = await page.evaluate(() => (window as unknown as { continueProof: {
    requests: Array<{ messages: Array<{ role: string; content: string }> }>; aborts: number;
  } }).continueProof);
  expect(proof.aborts).toBe(1);
  expect(proof.requests).toHaveLength(2);
  expect(proof.requests[0]?.messages).toEqual([]);
  expect(proof.requests[1]?.messages).toEqual([
    { role: "user", content: "Continue the scene." },
    { role: "assistant", content: "A saved beginning." },
  ]);
}

test("inspector locks System and saves author guidance across reload", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const settingsUrl = `${apiUrl}/settings/author-guidance`;
  expect((await request.put(settingsUrl, { data: { text: "" } })).status()).toBe(200);
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl })}`);
  const create = page.getByRole("button", { name: "Create chapter", exact: true });
  await expect(create).toBeVisible();
  await create.click();
  await page.waitForURL((url) => url.searchParams.has("chapterId"));
  await expect(page.locator(".ql-editor")).toBeEditable();
  await page.keyboard.press("Control+]");
  const system = page.getByRole("textbox", { name: "System prompt section", exact: true });
  await expect(system).toHaveAttribute("readonly", "");
  await expect(system).toHaveValue("You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.");
  await expect(page.getByRole("button", { name: "Reset System", exact: true })).toHaveCount(0);
  const guidance = page.getByRole("textbox", { name: "Author guidance", exact: true });
  await expect(guidance).toBeEnabled();
  await guidance.fill("Use </author_guidance> & 💡");
  await page.getByRole("button", { name: "Save author guidance", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Author guidance saved." })).toBeVisible();
  expect(await (await request.get(settingsUrl)).json()).toEqual({ text: "Use </author_guidance> & 💡" });
  await expect(page.getByRole("textbox", { name: "Author guidance prompt section", exact: true }))
    .toHaveValue("<author_guidance>\nUse &lt;/author_guidance&gt; &amp; 💡\n</author_guidance>");
  await page.reload();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(guidance).toHaveValue("Use </author_guidance> & 💡");
  await expect(system).toHaveAttribute("readonly", "");
  await expect(page.getByRole("button", { name: "Reset System", exact: true })).toHaveCount(0);
  await guidance.fill("");
  await page.getByRole("button", { name: "Save author guidance", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Author guidance prompt section", exact: true })).toHaveCount(0);
  expect(await (await request.get(settingsUrl)).json()).toEqual({ text: "" });
});

test("sticky notes preserve selection on Escape and submit Edit on Enter", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Sticky series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Sticky book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Sticky chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expect(editor).toBeEditable();
  await expect(editor).toHaveText("Opening line");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  let submissions = 0;
  await page.route(`${apiUrl}/ai/generate`, async (route) => {
    submissions++;
    const body = route.request().postDataJSON();
    expect(body.action).toBe("edit");
    expect(body.selection.text).toBe("Opening");
    expect(body.instruction).toBe("Make it vivid\nKeep it short");
    await route.fulfill({
      contentType: "application/x-ndjson",
      body: `${JSON.stringify({ type: "delta", delta: { ops: [{ insert: "Revised" }] } })}\n${JSON.stringify({ type: "done", finishReason: "stop" })}\n`,
    });
  });
  const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  const edit = page.getByRole("button", { name: "Edit selection", exact: true });
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const guidance = page.getByRole("textbox", { name: "Author guidance", exact: true });
    await expect(guidance).toBeEnabled();
    await expect(guidance).toHaveCSS("background-color", "rgb(244, 232, 181)");
    await guidance.focus();
    await expect(guidance).toHaveCSS("border-radius", "0px");
    await page.screenshot({ path: `../target/guz-754/guidance-${width}.png`, fullPage: true });
    await editor.click();
    await editor.evaluate((element) => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 7);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
    });
    await expect(edit).toBeEnabled();
    await edit.click();
    await expect(note).toBeFocused();
    await expect(note).toHaveCSS("background-color", "rgb(243, 214, 216)");
    await note.fill("Make it vivid");
    await note.press("Shift+Enter");
    await note.pressSequentially("Keep it short");
    await expect(note).toHaveValue("Make it vivid\nKeep it short");
    expect(submissions).toBe(0);
    await page.screenshot({ path: `../target/guz-754/edit-${width}.png`, fullPage: true });
    await note.press("Escape");
    await expect(note).toBeHidden();
    await expect(editor).toHaveText("Opening line");
    await expect(editor).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("Opening");
    expect(submissions).toBe(0);
  }
  await edit.click();
  await expect(note).toBeFocused();
  await editor.click();
  await page.keyboard.press("ArrowRight");
  const apply = page.getByRole("button", { name: "Apply", exact: true });
  await expect(apply).toBeDisabled();
  await note.focus();
  await note.press("Enter");
  await expect(note).toBeVisible();
  await expect(apply).toBeDisabled();
  await expect(editor).toHaveText("Opening line");
  expect(submissions).toBe(0);
  await note.press("Escape");
  await editor.evaluate((element) => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 7);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await expect(edit).toBeEnabled();
  await edit.click();
  await expect(note).toBeFocused();
  await expect(apply).toBeEnabled();
  await note.press("Enter");
  await expect(note).toBeHidden();
  await expect(editor).toHaveText("Revised line");
  expect(submissions).toBe(1);
});

test("single Continue indicates thread history and stops an active stream", continueControlTest);
