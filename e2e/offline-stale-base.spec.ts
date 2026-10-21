import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("offline tab rebases a stale submission without losing either tab's edits", async ({ context, page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");

  const seriesResponse = await request.post(`${apiUrl}/series`, {
    data: { title: "Offline series", position: 0 },
  });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, {
    data: { title: "Offline book", position: 0 },
  });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, {
    data: {
      title: "Offline chapter", position: 0, kind: "chapter",
      content: { ops: [{ insert: "Opening line\n" }] },
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
  const second = await context.newPage();
  const failures: string[] = [];
  for (const tab of [page, second]) {
    tab.on("pageerror", (error) => failures.push(error.message));
    tab.on("response", (response) => {
      if (response.url().startsWith(apiUrl) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.url()}`);
      }
    });
    await tab.setViewportSize({ width: 1100, height: 1000 });
    await tab.goto(`/?${query}`);
    await expectEditorReady(tab.locator(".ql-editor"));
    await expect(tab.locator(".ql-editor")).toHaveText("Opening line");
    await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  }

  let release!: () => void;
  const offline = new Promise<void>((resolve) => { release = resolve; });
  let heldBase: number | null = null;
  await page.route(`${deltaUrl}*`, async (route) => {
    if (route.request().method() === "GET") return route.abort("internetdisconnected");
    if (route.request().method() !== "POST") return route.continue();
    heldBase = route.request().postDataJSON().baseRev;
    await offline;
    await route.continue();
  });
  const editor = page.locator(".ql-editor");
  await editor.evaluate((element) => {
    (element as HTMLElement).focus();
    const range = document.createRange();
    range.setStart(element.querySelector("p")!.firstChild!, 0);
    range.collapse(true);
    window.getSelection()!.removeAllRanges();
    window.getSelection()!.addRange(range);
  });
  await page.keyboard.insertText("A offline ");
  await expect.poll(() => heldBase).toBe(chapter.head_rev);
  await page.keyboard.insertText("A pending ");
  await expect(editor).toHaveText("A offline A pending Opening line");

  let remoteText = "Opening line";
  for (const [index, edit] of [" B-one", " B-two", " B-three"].entries()) {
    await second.locator(".ql-editor").evaluate((element) => {
      (element as HTMLElement).focus();
      const text = element.querySelector("p")!.firstChild!;
      const range = document.createRange();
      range.setStart(text, text.textContent!.length);
      range.collapse(true);
      window.getSelection()!.removeAllRanges();
      window.getSelection()!.addRange(range);
    });
    const committed = second.waitForResponse((response) => (
      response.url() === deltaUrl && response.request().method() === "POST"
    ));
    await second.keyboard.insertText(edit);
    const response = await committed;
    expect(response.status()).toBe(200);
    expect((await response.json()).rev).toBe(chapter.head_rev + index + 1);
    remoteText += edit;
    await expect(second.locator(".ql-editor")).toHaveText(remoteText);
    await expect(second.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  }
  const beforeReconnect = await request.get(chapterUrl);
  expect(beforeReconnect.status()).toBe(200);
  const remote = await beforeReconnect.json();
  expect(remote.head_rev).toBe(chapter.head_rev + 3);
  expect(remote.content).toEqual({ ops: [{ insert: `${remoteText}\n` }] });
  expect(heldBase).toBeLessThan(remote.head_rev);
  await expect(editor).toHaveText("A offline A pending Opening line");

  const stalePost = page.waitForResponse((response) => (
    response.url() === deltaUrl && response.request().method() === "POST" &&
    response.request().postDataJSON().baseRev === heldBase
  ));
  release();
  const accepted = await stalePost;
  await page.unroute(`${deltaUrl}*`);
  expect(accepted.status()).toBe(200);
  expect((await accepted.json()).rev).toBe(remote.head_rev + 1);

  const expected = `A offline A pending ${remoteText}`;
  for (const tab of [page, second]) {
    await expect(tab.locator(".ql-editor")).toHaveText(expected);
    await expect(tab.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  }
  const savedResponse = await request.get(chapterUrl);
  expect(savedResponse.status()).toBe(200);
  const saved = await savedResponse.json();
  expect(saved.head_rev).toBe(chapter.head_rev + 5);
  expect(saved.content).toEqual({ ops: [{ insert: `${expected}\n` }] });
  expect(await editor.innerText()).toBe(expected);
  expect(await second.locator(".ql-editor").innerText()).toBe(expected);
  expect(failures).toEqual([]);
});
