import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openChapter(page: Page, request: APIRequestContext, width: number, action = "Continue") {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!apiUrl || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identity = `${action === "Edit" ? "GUZ865-commentary" : "GUZ807-Stop"}-${crypto.randomUUID()}`;
  await page.setViewportSize({ width, height: 1000 });
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Shortcut series", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Shortcut book", position: 0 } })).json();
  const chapter = await (await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Shortcut chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `${identity}\nOpening line\n` }] },
  } })).json();
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".editor-canvas .ql-editor");
  const edit = page.getByRole("button", { name: "Edit selection", exact: true });
  const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  const dialog = page.getByRole("dialog", { name: "Edit instruction", exact: true });
  const continueButton = page.getByRole("button", { name: "Continue writing", exact: true });
  const stop = page.getByRole("button", { name: "Stop AI", exact: true });
  await expectEditorReady(editor);
  await expect(editor.locator("p")).toHaveText([identity, "Opening line"]);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const before = (await (await request.get(`${provider}/control`)).json()).length as number;
  expect((await request.post(`${provider}/control/arm`, { data: { identity, model: action === "Edit" ? "e2e-edit" : "e2e-continue" } })).status()).toBe(200);
  await selectText(editor, 0);
  return { editor, edit, note, dialog, continueButton, stop, provider, before, control: `${provider}/control/${before}` };
}

async function selectText(editor: Locator, length: number) {
  await editor.click();
  await editor.evaluate((element, size) => {
    const text = element.querySelectorAll("p")[1]?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, size);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  }, length);
}

for (const width of [1100, 1440]) {
  test(`Continue shortcut starts and inserts text at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(run.stop).toBeVisible();
    await expect.poll(async () => (await request.get(`${run.control}/request`)).status()).toBe(200);
    expect((await request.post(`${run.control}/fragment`, { data: { text: "Generated text" } })).status()).toBe(200);
    await expect(run.editor).toContainText("Generated text");
    expect((await request.post(`${run.control}/finish`)).status()).toBe(200);
    await expect(run.stop).toBeHidden();
  });

  test(`Continue shortcut stops an existing run at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await run.continueButton.click();
    await expect(run.stop).toBeVisible();
    await expect.poll(async () => (await request.get(`${run.control}/request`)).status()).toBe(200);
    await run.editor.focus();
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(run.stop).toBeHidden();
    await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
    await expect.poll(async () => (await (await request.get(`${run.control}/closed`)).json()).closed).toBe(true);
    expect((await (await request.get(`${run.control}/closed`)).json()).completed).toBe(false);
  });

  test(`Edit shortcut opens and focuses instructions at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await selectText(run.editor, 7);
    await expect(run.edit).toBeEnabled();
    await page.keyboard.press("ControlOrMeta+K");
    await expect(run.dialog).toBeVisible();
    await expect(run.note).toBeFocused();
  });

  test(`Edit shortcut without selection retains browser default at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await expect(run.edit).toBeDisabled();
    await run.editor.evaluate(element => {
      window.addEventListener("keydown", event => {
        if (event.key.toLowerCase() === "k") element.setAttribute("data-shortcut-prevented", String(event.defaultPrevented));
      });
    });
    await page.keyboard.press("ControlOrMeta+K");
    await expect(run.dialog).toBeHidden();
    await expect(run.editor).toHaveAttribute("data-shortcut-prevented", "false");
  });

  test(`Instruction shortcut applies Edit without Continue at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width, "Edit");
    await selectText(run.editor, 7);
    await expect(run.edit).toBeEnabled();
    await run.edit.click();
    await run.note.fill("Fix grammar");
    await run.note.press("Shift+Enter");
    await expect(run.note).toHaveValue("Fix grammar\n");
    await run.note.press("ControlOrMeta+Enter");
    await expect.poll(async () => (await request.get(`${run.control}/request`)).status()).toBe(200);
    expect((await (await request.get(`${run.control}/request`)).json()).model).toBe("e2e-edit");
    expect((await request.post(`${run.control}/fragment`, { data: { text: "<replacement>Revised</replacement>" } })).status()).toBe(200);
    expect((await request.post(`${run.control}/finish`)).status()).toBe(200);
    await expect(run.editor).toContainText("Revised line");
    await expect(run.stop).toBeHidden();
    expect((await (await request.get(`${run.provider}/control`)).json()).length).toBe(run.before + 1);
  });

  for (const focus of ["body", "rail"]) {
    test(`Continue shortcut uses saved selection from ${focus} at ${width}px`, async ({ page, request }) => {
      const run = await openChapter(page, request, width);
      await selectText(run.editor, 7);
      await expect(run.edit).toBeEnabled();
      const target = focus === "rail" ? page.getByRole("button", { name: "Context inspector", exact: true }) : page.locator("body");
      if (focus === "rail") await target.click();
      else await target.evaluate(element => element.setAttribute("tabindex", "-1"));
      await target.focus();
      await expect(target).toBeFocused();
      await page.keyboard.press("ControlOrMeta+Enter");
      await expect(run.stop).toBeVisible();
      await expect.poll(async () => (await request.get(`${run.control}/request`)).status()).toBe(200);
      expect((await request.post(`${run.control}/fragment`, { data: { text: "Saved selection text" } })).status()).toBe(200);
      await expect(run.editor).toContainText("Opening Saved selection text line");
      expect((await request.post(`${run.control}/finish`)).status()).toBe(200);
      await expect(run.stop).toBeHidden();
    });
  }

  test(`Lore modal buttons retain AI shortcut defaults at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await selectText(run.editor, 7);
    await expect(run.edit).toBeEnabled();
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const lore = page.getByRole("region", { name: "Series lore", exact: true });
    await lore.getByRole("button", { name: "New lore", exact: true }).click();
    const create = page.getByRole("dialog", { name: "Create lore", exact: true });
    await create.getByRole("textbox", { name: "Name", exact: true }).fill("Shortcut lore");
    await create.getByRole("button", { name: "Save lore", exact: true }).click();
    await lore.getByRole("button", { name: "Shortcut lore · character", exact: true }).click();
    const modal = page.getByRole("dialog", { name: "Edit lore", exact: true });
    await modal.evaluate(element => {
      window.addEventListener("keydown", event => {
        if (["enter", "k"].includes(event.key.toLowerCase())) element.setAttribute(`data-shortcut-${event.key.toLowerCase()}`, String(event.defaultPrevented));
      });
    });
    for (const key of ["Enter", "K"]) {
      const button = modal.getByRole("button", { name: /^(Delete lore|Confirm delete)$/ });
      await button.focus();
      await expect(button).toBeFocused();
      await page.keyboard.press(`ControlOrMeta+${key}`);
      await expect(run.stop).toBeHidden();
      await expect(run.dialog).toBeHidden();
      await expect(modal).toHaveAttribute(`data-shortcut-${key.toLowerCase()}`, "false");
    }
    expect((await (await request.get(`${run.provider}/control`)).json()).length).toBe(run.before);
  });

  test(`AI shortcuts expose accessibility metadata at ${width}px`, async ({ page, request }) => {
    const run = await openChapter(page, request, width);
    await expect(run.continueButton).toHaveAttribute("aria-keyshortcuts", "Control+Enter Meta+Enter");
    await expect(run.edit).toHaveAttribute("aria-keyshortcuts", "Control+K Meta+K");
    await run.continueButton.click();
    await expect(run.stop).toBeVisible();
    await expect(run.stop).toHaveAttribute("aria-keyshortcuts", "Control+Enter Meta+Enter");
    await expect.poll(async () => (await request.get(`${run.control}/request`)).status()).toBe(200);
    expect((await request.post(`${run.control}/finish`)).status()).toBe(200);
    await expect(run.stop).toBeHidden();
  });
}
