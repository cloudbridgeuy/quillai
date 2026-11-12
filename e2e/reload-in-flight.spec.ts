import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

test("reload during a save keeps server text and exports the full unsaved document", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const failures: string[] = [];
  page.on("pageerror", (error) => failures.push(error.message));
  page.on("response", (response) => {
    if (response.url().startsWith(apiUrl) && response.status() >= 400) {
      failures.push(`${response.status()} ${response.url()}`);
    }
  });
  const seriesResponse = await request.post(`${apiUrl}/series`, {
    data: { title: "Reload series", position: 0 },
  });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
    data: { title: "Reload book", position: 0 },
  });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const initialText = "Server opening line\n";
  const chapterResponse = await request.post(chaptersUrl, {
    data: {
      title: "Reload chapter", position: 0, kind: "chapter",
      content: { ops: [{ insert: initialText }] },
    },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  const chapterUrl = `${chaptersUrl}/${chapter.id}`;
  const deltaUrl = `${chapterUrl}/deltas`;
  const query = new URLSearchParams({
    apiBaseUrl: apiUrl,
    seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id),
  });
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${query}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(editor).toHaveText(initialText);
  await expectSaved(page);

  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  let sawPost!: () => void;
  const held = new Promise<void>((resolve) => { sawPost = resolve; });
  await page.route(deltaUrl, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    sawPost();
    await gate;
    await route.abort("aborted");
  });
  await editor.click();
  await page.keyboard.press("ControlOrMeta+End");
  await page.keyboard.insertText(" unsaved flight");
  await held;
  await page.keyboard.insertText(" and pending text");
  const unsavedText = "Server opening line unsaved flight and pending text\n";
  await expect(editor).toHaveText(unsavedText);
  await page.reload();
  release();
  await page.unrouteAll({ behavior: "wait" });
  await expectEditorReady(editor);
  await expectSaved(page);

  const serverResponse = await request.get(chapterUrl);
  expect(serverResponse.status()).toBe(200);
  const server = await serverResponse.json();
  expect(server.head_rev).toBe(chapter.head_rev);
  expect(server.content).toEqual({ ops: [{ insert: initialText }] });
  const serverText = server.content.ops.map((op: { insert: string }) => op.insert).join("");
  expect(`${await editor.innerText()}\n`).toBe(serverText);
  const alert = page.getByRole("alert").filter({ has: page.getByRole("link", { name: "Export recovery", exact: true }) });
  await expect(alert).toBeVisible();
  const recoveryLink = alert.getByRole("link", { name: "Export recovery", exact: true });
  const href = await recoveryLink.getAttribute("href");
  expect(href).toMatch(/^data:application\/json;charset=utf-8,/);
  const copies = JSON.parse(decodeURIComponent(href!.split(",")[1]!));
  expect(copies).toHaveLength(1);
  expect(JSON.parse(copies[0])).toEqual({
    scope: { seriesId: series.id, bookId: book.id, chapterId: chapter.id },
    document: { ops: [{ insert: unsavedText }] },
  });

  await editor.click();
  await page.keyboard.press("ControlOrMeta+End");
  const savedPost = page.waitForResponse((response) => (
    response.url() === deltaUrl && response.request().method() === "POST"
  ));
  await page.keyboard.insertText(" saved after reload");
  expect((await savedPost).status()).toBe(200);
  await expectSaved(page);
  const savedResponse = await request.get(chapterUrl);
  expect(savedResponse.status()).toBe(200);
  const saved = await savedResponse.json();
  const savedText = "Server opening line saved after reload\n";
  expect(saved.head_rev).toBe(chapter.head_rev + 1);
  expect(saved.content).toEqual({ ops: [{ insert: savedText }] });
  expect(`${await editor.innerText()}\n`).toBe(savedText);
  expect(failures).toEqual([]);
});
