import { expect, test } from "../app/node_modules/@playwright/test/index.js";

for (const width of [1100, 1440]) {
  test(`desktop lore highlights, cards and Continue window are visual only at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const series = await (await request.post(`${api}/series`, { data: { title: `Lore UI ${width}`, position: 0 } })).json();
    const base = `${api}/series/${series.id}`;
    const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
    const text = "Mira met Captain Mira beside the compass. Omitted waited.\n";
    const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
      title: "Lore UI", position: 0, kind: "chapter", content: { ops: [{ insert: text }] },
    } })).json();
    const secret = "HIDDEN_NOTES_MUST_NEVER_APPEAR_759";
    for (const data of [
      { name: "Mira", aliases: ["Captain Mira"], keywords: ["compass"], level: "known_dont_reveal", visible_text: "The ship's navigator." },
      { name: "Omitted", level: "leave_out", visible_text: "Not sent." },
      { name: "Always", pinned: true, visible_text: "Pinned context." },
      { name: "Too large", keywords: ["compass"], visible_text: "x".repeat(65536) },
    ]) {
      expect((await request.post(`${base}/lore`, { data: { kind: "character", hidden_notes: secret, ...data } })).status()).toBe(201);
    }
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    await expect(editor).toBeEditable();
    await expect(editor).toHaveText(text.trim());
    await expect.poll(() => page.evaluate(() => {
      const highlights = CSS.highlights.get("quillai-lore-known_dont_reveal");
      return highlights ? [...highlights].map(range => range.toString()) : [];
    })).toEqual(["Mira", "Captain Mira", "compass"]);
    const chapterUrl = `${base}/books/${book.id}/chapters/${chapter.id}`;
    const before = await (await request.get(chapterUrl)).json();
    const point = await page.evaluate(() => {
      const range = [...CSS.highlights.get("quillai-lore-known_dont_reveal")!][0]!;
      const rect = range.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    });
    await page.mouse.move(point.x, point.y);
    const card = page.getByRole("dialog", { name: "Lore match", exact: true });
    await expect(card).toBeVisible();
    await expect(card).toContainText("Mira");
    await expect(card).toContainText("character · Known, don't reveal");
    await expect(card).toContainText("The ship's navigator.");
    await expect(card).not.toContainText(secret);
    await page.screenshot({ path: `../target/guz-759/hover-${width}.png` });
    await page.keyboard.press("Escape");
    await expect(card).not.toBeVisible();
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const list = page.getByRole("region", { name: "Lore in window", exact: true });
    await expect(list).toContainText("Mira");
    await expect(list).toContainText("3 matches");
    await expect(list).toContainText("Always");
    await expect(list).toContainText("0 matches");
    await expect(list).not.toContainText("Omitted");
    await expect(list).not.toContainText("Too large");
    await expect(page.locator("body")).not.toContainText(secret);
    await list.getByRole("button", { name: /Mira/ }).click();
    await expect(card).toBeVisible();
    await page.screenshot({ path: `../target/guz-759/window-${width}.png` });
    await page.keyboard.press("Escape");
    const after = await (await request.get(chapterUrl)).json();
    expect(after.content).toEqual(before.content);
    expect(after.head_rev).toBe(before.head_rev);
    expect(after.content).toEqual({ ops: [{ insert: text }] });
    await editor.click();
    await page.keyboard.press("Control+End");
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("spinbutton", { name: "Window Size", exact: true }).fill("8");
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    await expect(list).toContainText("Always");
    await expect(list.getByRole("button", { name: /Mira/ })).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => CSS.highlights.get("quillai-lore-known_dont_reveal")?.size ?? 0)).toBe(0);
    await page.getByRole("button", { name: "Close context", exact: true }).click();
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("checkbox", { name: "Whole document", exact: true }).check();
    await page.keyboard.press("Escape");
    const bodies: string[] = [];
    await page.route("**/ai/generate", async route => {
      bodies.push(route.request().postData() ?? "");
      await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
    });
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    await expect.poll(() => bodies.length).toBe(1);
    expect(JSON.parse(bodies[0]!).lore.map((entry: { name: string }) => entry.name)).toEqual(["Always", "Mira"]);
    expect(bodies[0]).not.toContain(secret);
    await page.reload();
    await expect(editor).toHaveText(text.trim());
    await page.setViewportSize({ width: 1099, height: 1000 });
    await expect.poll(() => page.evaluate(() => CSS.highlights.has("quillai-lore-known_dont_reveal"))).toBe(false);
  });
}

test("series lore persists, matches near the caret, and excludes author-only notes from AI", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Lore series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Lore book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Lore chapter", position: 0, kind: "chapter" },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expect(editor).toBeEditable();
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const lore = page.getByRole("region", { name: "Series lore", exact: true });
  await lore.getByRole("button", { name: "New lore", exact: true }).click();
  await lore.getByLabel("Name", { exact: true }).fill("Mira");
  await lore.getByLabel("Aliases (one per line)").fill("Captain Mira\nThe captain");
  await lore.getByLabel("Keywords (one per line)").fill("navigator\ncompass");
  await lore.getByLabel("Visible text (AI context)").fill("Mira is the ship's navigator.");
  const secret = "AUTHOR_ONLY_MIRA_SECRET_671";
  await lore.getByLabel("Hidden notes — author-only, never sent to AI").fill(secret);
  await lore.getByLabel("In AI requests").selectOption("known_dont_reveal");
  await lore.getByRole("button", { name: "Save lore", exact: true }).click();
  await expect(lore.getByRole("button", { name: "Mira · character", exact: true })).toBeVisible();
  await lore.getByRole("button", { name: "Mira · character", exact: true }).click();
  await expect(lore.getByLabel("Hidden notes — author-only, never sent to AI")).toHaveValue(secret);
  await expect(lore.getByLabel("Aliases (one per line)")).toHaveValue("Captain Mira\nThe captain");
  await lore.getByLabel("Pin in AI context").check();
  await lore.getByRole("button", { name: "Save lore", exact: true }).click();
  await lore.getByRole("button", { name: "Mira · character · pinned", exact: true }).click();
  await lore.getByLabel("Pin in AI context").uncheck();
  await lore.getByRole("button", { name: "Save lore", exact: true }).click();
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.screenshot({ path: `../target/guz-671/lore-panel-${width}.png`, fullPage: true });
  }
  await page.getByRole("button", { name: "Close manuscript", exact: true }).click();
  await editor.click();
  await page.keyboard.type("Mira studied the compass.");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const prompt = page.getByRole("textbox", { name: "Lore prompt section", exact: true });
  await expect(prompt).toHaveValue(/Mira is the ship's navigator/);
  await expect(prompt).toHaveValue(/MUST NOT be revealed/);
  await expect(prompt).not.toHaveValue(new RegExp(secret));
  const injected = page.getByRole("region", { name: "Injected lore", exact: true });
  await expect(injected).toContainText("Mira · character · keyword · Known, don't reveal");
  await expect(injected).toContainText("estimated tokens");
  await page.reload();
  await expect(editor).toHaveText("Mira studied the compass.");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(prompt).toHaveValue(/Mira is the ship's navigator/);
  await expect(injected).toContainText("Mira · character · keyword");
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await injected.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `../target/guz-671/inspector-${width}.png`, fullPage: true });
  }
  await page.getByRole("button", { name: "Close context", exact: true }).click();
  const bodies: string[] = [];
  await page.route("**/ai/generate", async route => {
    bodies.push(route.request().postData() ?? "");
    await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
  });
  await editor.click();
  await page.keyboard.press("Control+End");
  await page.getByRole("button", { name: "Continue writing", exact: true }).click();
  await expect.poll(() => bodies.length).toBe(1);
  await expect(page.getByRole("button", { name: /^Continue writing/ })).toBeEnabled();
  expect(bodies[0]).not.toContain(secret);
  expect(JSON.parse(bodies[0]!).lore).toEqual([{ name: "Mira", kind: "character", visible_text: "Mira is the ship's navigator.", level: "known_dont_reveal" }]);
  await editor.selectText();
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect.poll(() => bodies.length).toBe(2);
  expect(bodies[1]).not.toContain(secret);
  expect(JSON.parse(bodies[1]!).action).toBe("edit");
  expect(JSON.parse(bodies[1]!).lore).toHaveLength(1);
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await lore.getByRole("button", { name: "Mira · character", exact: true }).click();
  await lore.getByRole("button", { name: "Delete lore", exact: true }).click();
  await expect(lore.getByText("No lore yet.", { exact: true })).toBeVisible();
  expect(await (await request.get(`${apiUrl}/series/${series.id}/lore`)).json()).toEqual([]);
});
