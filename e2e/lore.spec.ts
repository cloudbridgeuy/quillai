import { expect, test } from "../app/node_modules/@playwright/test/index.js";

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
  await page.reload();
  await expect(editor).toHaveText("Mira studied the compass.");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(prompt).toHaveValue(/Mira is the ship's navigator/);
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
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
