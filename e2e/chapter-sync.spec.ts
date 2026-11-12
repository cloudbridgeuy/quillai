import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectSaved } from "./save-state";

test("two pages save, sync, and reload one chapter", async ({ context, page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const seriesResponse = await request.post(`${apiUrl}/series`, {
    data: { title: "E2E series", position: 0 },
  });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
    data: { title: "E2E book", position: 0 },
  });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: {
      title: `E2E chapter ${test.info().project.name}`, position: 0, kind: "chapter",
      content: { ops: [{ insert: "Opening line\n" }] },
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
  const failures: string[] = [];
  for (const tab of [page, second]) {
    tab.on("pageerror", (error) => failures.push(error.message));
    tab.on("response", (response) => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.url()}`);
      }
    });
    await tab.goto(`/?${query}`);
    await expect(tab.locator(".ql-editor")).toHaveText("Opening line");
    await expect(tab.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
    await expectSaved(tab);
    await expect(tab.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
  }

  await page.setViewportSize({ width: 1440, height: 1000 });
  const title = page.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true });
  await expect(title).toHaveCSS("font-size", "13px");
  await expect(title).toHaveCSS("font-weight", "600");
  await expect(title).toHaveCSS("letter-spacing", "2.6px");
  await expect(title).toHaveCSS("text-transform", "uppercase");
  await expect(title).toHaveCSS("text-align", "center");
  expect(await title.evaluate((element) => (element as HTMLElement).isContentEditable)).toBe(false);
  await expect(title).toHaveCSS("color", "rgb(78, 98, 114)");
  const editor = page.locator(".ql-editor");
  await editor.click();
  await editor.evaluate((element) => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 7);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  });
  await expect(page.getByRole("button", { name: "Edit selection", exact: true })).toBeEnabled();
  const selectedText = await page.evaluate(() => window.getSelection()?.toString());
  expect(selectedText).toBe("Opening");
  for (const shortcut of ["Control+]", "Meta+]"]) {
    await page.keyboard.press(shortcut);
    await expect(page.getByRole("heading", { name: "Live Prompt Context" })).toBeVisible();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selectedText);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("heading", { name: "Live Prompt Context" })).toBeHidden();
    await expect(editor).toBeFocused();
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selectedText);
  }
  const inspector = page.getByRole("button", { name: "Context inspector", exact: true });
  await inspector.hover();
  await expect(inspector).toHaveAttribute("aria-expanded", "false");
  await inspector.click();
  await expect(page.getByRole("heading", { name: "Live Prompt Context" })).toBeVisible();
  await expect(editor).toBeFocused();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selectedText);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+[");
  await expect(page.getByRole("heading", { name: "Manuscript", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+]");
  await page.getByRole("textbox", { name: "Author guidance", exact: true }).focus();
  await page.keyboard.press("Escape");
  await expect(editor).toBeFocused();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selectedText);
  await page.screenshot({ path: `../target/guz-750/${test.info().project.name}/editor-desktop.png`, fullPage: true });
  for (const width of [768, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.keyboard.press("Control+]");
    const drawer = page.locator("#context-drawer");
    await expect(drawer).toBeVisible();
    const drawerBounds = await drawer.boundingBox();
    const pageBounds = await page.locator(".editor-page").boundingBox();
    expect(drawerBounds).not.toBeNull();
    expect(pageBounds).not.toBeNull();
    expect(drawerBounds!.y + drawerBounds!.height).toBeLessThanOrEqual(pageBounds!.y);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.keyboard.press("Escape");
  }
  await page.setViewportSize({ width: 1440, height: 1000 });

  await page.locator(".ql-editor").fill("Page one writes");
  await expect(second.locator(".ql-editor")).toHaveText("Page one writes");
  await expectSaved(page);
  await expect(page.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
  const firstSaved = await request.get(chapterUrl);
  expect(firstSaved.ok()).toBe(true);
  const firstHead = (await firstSaved.json()).head_rev;
  expect(firstHead).toBeGreaterThan(chapter.head_rev);

  await second.locator(".ql-editor").fill("Page one writes; page two replies");
  await expect(page.locator(".ql-editor")).toHaveText("Page one writes; page two replies");
  await expectSaved(second);
  await expect(second.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
  const savedResponse = await request.get(chapterUrl);
  expect(savedResponse.ok()).toBe(true);
  const saved = await savedResponse.json();
  expect(saved.head_rev).toBeGreaterThan(firstHead);
  expect(saved.content).toEqual({ ops: [{ insert: "Page one writes; page two replies\n" }] });

  for (const tab of [page, second]) {
    await tab.reload();
    await expect(tab.locator(".ql-editor")).toHaveText("Page one writes; page two replies");
    await expect(tab.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
    await expectSaved(tab);
    await expect(tab.getByRole("heading", { name: `E2E chapter ${test.info().project.name}`, exact: true })).toBeVisible();
  }
  expect(failures).toEqual([]);
});

test("tree opens a second chapter only after saving and keeps text when saving fails", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Switch series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Switch book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapters = [];
  for (const [position, title, text] of [[0, `Switch first ${test.info().project.name}`, "First opening\n"], [1, `Switch second ${test.info().project.name}`, "Second opening\n"]] as const) {
    const response = await request.post(chaptersUrl, {
      data: { title, position, kind: "chapter", content: { ops: [{ insert: text }] } },
    });
    expect(response.status()).toBe(201);
    chapters.push(await response.json());
  }
  const [first, second] = chapters;
  const query = new URLSearchParams({
    apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(first.id),
  });
  let secondGets = 0;
  let firstGets = 0;
  page.on("request", (request) => {
    if (request.method() === "GET" && request.url() === `${chaptersUrl}/${second.id}`) secondGets++;
    if (request.method() === "GET" && request.url() === `${chaptersUrl}/${first.id}`) firstGets++;
  });
  await page.goto(`/?${query}`);
  const editor = page.locator(".ql-editor");
  await expect(editor).toHaveText("First opening");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const firstButton = page.getByRole("button", { name: `Switch first ${test.info().project.name}`, exact: true });
  const secondButton = page.getByRole("button", { name: `Switch second ${test.info().project.name}`, exact: true });
  await expect(firstButton).toHaveAttribute("aria-pressed", "true");
  let releaseSave!: () => void;
  const saveGate = new Promise<void>((resolve) => { releaseSave = resolve; });
  let sawSave!: () => void;
  const saving = new Promise<void>((resolve) => { sawSave = resolve; });
  await page.route(`${chaptersUrl}/${first.id}/deltas`, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    sawSave();
    await saveGate;
    await route.continue();
  });
  await editor.fill("First pending edit");
  await secondButton.click();
  await saving;
  await expect(secondButton).toBeDisabled();
  await expect(editor).toHaveAttribute("contenteditable", "false");
  await expect(page.getByRole("heading", { name: `Switch first ${test.info().project.name}`, exact: true })).toBeVisible();
  expect(secondGets).toBe(0);
  releaseSave();
  await expect(editor).toHaveText("Second opening");
  await expect(page.getByRole("heading", { name: `Switch second ${test.info().project.name}`, exact: true })).toBeVisible();
  await expect(secondButton).toHaveAttribute("aria-pressed", "true");
  await expect(firstButton).toHaveAttribute("aria-pressed", "false");
  await expect(page).toHaveURL(new RegExp(`chapterId=${second.id}(?:&|$)`));
  expect(secondGets).toBe(1);
  const saved = await request.get(`${chaptersUrl}/${first.id}`);
  expect((await saved.json()).content).toEqual({ ops: [{ insert: "First pending edit\n" }] });

  let releaseFailure!: () => void;
  const failureGate = new Promise<void>((resolve) => { releaseFailure = resolve; });
  let sawFailure!: () => void;
  const failing = new Promise<void>((resolve) => { sawFailure = resolve; });
  await page.route(`${chaptersUrl}/${second.id}/deltas`, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    sawFailure();
    await failureGate;
    await route.fulfill({ status: 503, json: { error: "Save unavailable", code: "UNAVAILABLE" } });
  });
  await editor.fill("Second unsaved text retained");
  await firstButton.click();
  await failing;
  releaseFailure();
  await expect(page.getByRole("alert").filter({ hasText: "Chapter switch failed" })).toContainText("Save stopped");
  const failedSave = page.getByRole("alert", { name: "Chapter save status", exact: true });
  await expect(failedSave).toBeVisible();
  await expect(failedSave).toHaveAttribute("aria-live", "assertive");
  await expect(failedSave).toHaveCSS("background-color", "rgb(180, 65, 47)");
  await expect(failedSave).toHaveCSS("color", "rgb(255, 255, 255)");
  await expect(page.locator("#manuscript-drawer").getByRole("link", { name: "Download backup", exact: true })).toBeVisible();
  await expect(editor).toHaveText("Second unsaved text retained");
  await expect(page.getByRole("heading", { name: `Switch second ${test.info().project.name}`, exact: true })).toBeVisible();
  await expect(secondButton).toHaveAttribute("aria-pressed", "true");
  await expect(page).toHaveURL(new RegExp(`chapterId=${second.id}(?:&|$)`));
  expect(firstGets).toBe(1);
  const recovery = await page.evaluate(() => window.sessionStorage.getItem("quillai.chapter.recovery.v1"));
  expect(JSON.parse(recovery!).document.ops).toEqual([{ insert: "Second unsaved text retained\n" }]);
});
