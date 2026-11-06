import { expect, test, type Page, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

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
  await expectEditorReady(editor);
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
    await page.screenshot({ path: `../target/guz-755/${test.info().project.name}/idle-${width}.png`, fullPage: true });
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(page.locator("#manuscript-drawer").getByRole("link", { name: "Download Markdown", exact: true })).toBeVisible();
    await page.screenshot({ path: `../target/guz-755/${test.info().project.name}/manuscript-${width}.png`, fullPage: true });
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  }
  await continueButton.click();
  await expect(editor).toHaveText("A saved beginning.");
  await expect(continueButton).toHaveAccessibleName("Continue writing from a cut-off reply");
  await expect(continueButton.getByText("from a cut-off reply", { exact: true })).toBeVisible();
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const context = page.locator("#context-drawer");
  await expect(context.getByRole("textbox", { name: "Story Context", exact: true })).toBeVisible();
  await expect(context.getByRole("textbox", { name: "Continue instruction", exact: true })).toBeVisible();
  await expect(context.getByRole("spinbutton", { name: "Window Size", exact: true })).toHaveCount(0);
  await expect(context.getByRole("checkbox", { name: "Whole document", exact: true })).toHaveCount(0);
  await expect(context.getByRole("button", { name: "Discard", exact: true })).toBeEnabled();
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-755/${test.info().project.name}/with-thread-${width}.png`, fullPage: true });
  }
  await expect(page.getByRole("option", { name: "Continue · from a cut-off reply", exact: true })).toHaveCount(1);
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await continueButton.click();
  await expect(continueButton).toHaveAccessibleName("Stop AI");
  await expect(continueButton.getByText("from a cut-off reply", { exact: true })).toBeVisible();
  await expect(editor).toContainText("More words.");
  await continueButton.click();
  await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
  await expectEditorReady(editor);
  await expect(editor).toHaveText("A saved beginning.");
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
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: `Inspector ${test.info().project.name}`, position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Inspector book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: { title: "Inspector chapter", position: 0, kind: "chapter" } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
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
  await expectEditorReady(editor);
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
    await page.screenshot({ path: `../target/guz-754/${test.info().project.name}/guidance-${width}.png`, fullPage: true });
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
    await page.screenshot({ path: `../target/guz-754/${test.info().project.name}/edit-${width}.png`, fullPage: true });
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

test("Edit and Continue send only their own instructions", async ({ page, request }) => {
  await page.setViewportSize({ width: 1100, height: 1000 });
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Instruction isolation", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Instructions", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Separate instructions", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  const bodies: Array<Record<string, unknown>> = [];
  await page.route(`${apiUrl}/ai/generate`, async (route) => {
    const body = route.request().postDataJSON();
    bodies.push(body);
    await route.fulfill({
      contentType: "application/x-ndjson",
      body: `${JSON.stringify({ type: "delta", delta: { ops: [{ insert: body.action === "edit" ? "Revised" : "The story continues." }] } })}\n${JSON.stringify({ type: "done", finishReason: "stop" })}\n`,
    });
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  const saved = page.getByRole("status", { name: "Chapter save status", exact: true });
  await expectEditorReady(editor);
  await expect(editor).toHaveText("Opening line");
  await expect(saved).toHaveText("Saved");
  const contextToggle = page.getByRole("button", { name: "Context inspector", exact: true });
  await contextToggle.click();
  const continueNote = page.getByRole("textbox", { name: "Continue instruction", exact: true });
  await continueNote.fill("Introduce a mysterious visitor");
  await contextToggle.click();
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
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  const editNote = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  await expect(editNote).toHaveValue("");
  await editNote.fill("Replace passive verbs with active verbs");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(editNote).toBeHidden();
  await expect(editor).toHaveText("Revised line");
  await expect(saved).toHaveText("Saved");
  expect(bodies).toHaveLength(1);
  expect(bodies[0]?.action).toBe("edit");
  expect(bodies[0]?.instruction).toBe("Replace passive verbs with active verbs");
  expect(JSON.stringify(bodies[0])).not.toContain("Introduce a mysterious visitor");
  await contextToggle.click();
  await expect(continueNote).toHaveValue("Introduce a mysterious visitor");
  await contextToggle.click();
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect(editor).toContainText("The story continues.");
  await expect(saved).toHaveText("Saved");
  expect(bodies).toHaveLength(2);
  expect(bodies[1]?.action).toBe("generate");
  expect(bodies[1]?.instruction).toBe("Introduce a mysterious visitor");
  expect(JSON.stringify(bodies[1])).not.toContain("Replace passive verbs with active verbs");
});

test("settings persist outside the sheet and failed saves show the red recovery pill", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Settings series", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Settings book", position: 0 } })).json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: { title: "Settings chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "The harbour was quiet.\n".repeat(30) }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  const status = page.getByRole("status", { name: "Chapter save status", exact: true });
  const gear = page.getByRole("button", { name: "Settings", exact: true });
  const settings = page.getByRole("dialog", { name: "Settings", exact: true });
  await expect(status).toHaveText("Saved");
  await expectEditorReady(editor);
  await editor.click();
  await editor.evaluate((element) => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 3);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await gear.click();
  await expect(settings).toBeVisible();
  await settings.getByRole("spinbutton", { name: "Window Size", exact: true }).fill("2400");
  await settings.getByRole("checkbox", { name: "Whole document", exact: true }).check();
  await settings.getByRole("checkbox", { name: "Tint AI text", exact: true }).uncheck();
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
  await expect(editor).toBeFocused();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("The");
  await page.reload();
  await expect(status).toHaveText("Saved");
  await gear.click();
  await expect(settings.getByRole("spinbutton", { name: "Window Size", exact: true })).toHaveValue("2400");
  await expect(settings.getByRole("checkbox", { name: "Whole document", exact: true })).toBeChecked();
  await expect(settings.getByRole("checkbox", { name: "Tint AI text", exact: true })).not.toBeChecked();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(settings).toBeHidden();
  await expect(page.locator("#context-drawer")).toBeVisible();
  await gear.click();
  await expect(page.locator("#context-drawer")).toBeHidden();
  await page.locator(".editor-chapter-title").click();
  await expect(settings).toBeHidden();
  const savedSheetTops = new Map<number, number>();
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.evaluate(() => window.scrollTo(0, 0));
    await expect(status).toBeInViewport();
    const sheet = await page.locator(".editor-page").boundingBox();
    const label = await status.boundingBox();
    if (!sheet || !label) throw new Error("Missing sheet or status");
    savedSheetTops.set(width, sheet.y);
    expect(label.y + label.height).toBeLessThan(sheet.y);
    expect(Math.abs(label.x + label.width / 2 - sheet.x - sheet.width / 2)).toBeLessThan(1);
    await page.screenshot({ path: `../target/guz-758/${test.info().project.name}/saved-${width}.png` });
    await gear.click();
    await expect(settings).toBeVisible();
    await expect(status).toBeInViewport();
    const prose = await editor.boundingBox();
    const popover = await settings.boundingBox();
    if (!prose || !popover) throw new Error("Missing prose or settings");
    expect(prose.x + prose.width).toBeLessThan(popover.x);
    await page.screenshot({ path: `../target/guz-758/${test.info().project.name}/gear-${width}.png` });
    await page.getByRole("button", { name: "Settings", exact: true }).click();
  }
  await page.evaluate(() => window.scrollTo(0, 400));
  const scrolledStatus = await status.boundingBox();
  expect(scrolledStatus?.y).toBeLessThan(0);
  await page.evaluate(() => window.scrollTo(0, 0));
  let saveAttempts = 0;
  await page.route(`${chaptersUrl}/${chapter.id}/deltas`, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    saveAttempts++;
    await route.fulfill({ status: 503, json: { error: "Save unavailable", code: "UNAVAILABLE" } });
  });
  await editor.click();
  await page.keyboard.type("Local work");
  const failureDetail = "Save stopped: Something went wrong. Try again. Local work retained; polling will retry.";
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true }).locator("span").first()).toHaveText("Not saved");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveAttribute("title", failureDetail);
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveAccessibleDescription(failureDetail);
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true }).locator("#chapter-save-detail")).toHaveText(failureDetail);
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true }).locator("#chapter-save-detail")).toHaveCSS("position", "absolute");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true }).locator("#chapter-save-detail")).toHaveCSS("width", "1px");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveClass(/editor-save-failed/);
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveAttribute("aria-live", "assertive");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveCSS("background-color", "rgb(180, 65, 47)");
  await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toHaveCSS("color", "rgb(255, 255, 255)");
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.evaluate(() => window.scrollTo(0, 0));
    const sheet = await page.locator(".editor-page").boundingBox();
    const pill = await page.getByRole("alert", { name: "Chapter save status", exact: true }).boundingBox();
    if (!sheet || !pill) throw new Error("Missing sheet or failure pill");
    const savedTop = savedSheetTops.get(width);
    if (savedTop === undefined) throw new Error("Missing saved sheet top");
    expect(Math.abs(sheet.y - savedTop)).toBeLessThanOrEqual(1);
    console.log(JSON.stringify({ width, savedSheetTop: savedTop, failedSheetTop: sheet.y }));
    expect(pill.y + pill.height).toBeLessThan(sheet.y);
    expect(pill.x).toBeGreaterThanOrEqual(56);
    expect(pill.x + pill.width).toBeLessThanOrEqual(width - 56);
    await page.screenshot({ path: `../target/guz-758/${test.info().project.name}/failed-${width}.png` });
  }
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await expect(page.getByRole("region", { name: "Manuscript outline", exact: true }).getByRole("button", { name: "Settings chapter", exact: true })).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download Markdown", exact: true }).click();
  expect((await download).suggestedFilename()).toBe("settings-chapter.md");
  expect(saveAttempts).toBeGreaterThanOrEqual(1);
});
