import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

const original = "He try to distract himself.";
const replacement = "He tried to distract himself.";

async function openEditChapter(page: Page, request: APIRequestContext, identity: string, wholeParagraph = false, error?: string) {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  const providerUrl = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!apiUrl || !providerUrl) throw new Error("Run with cargo xtask e2e and its default catalog");
  const failures: string[] = [];
  page.on("pageerror", error => failures.push(error.message));
  page.on("response", response => {
    if (response.url().startsWith(apiUrl) && response.status() >= 400) {
      failures.push(`${response.status()} ${response.request().method()} ${response.url()}`);
    }
  });
  const series = await (await request.post(`${apiUrl}/series`, { data: { title: "Edit replacement series", position: 0 } })).json();
  const book = await (await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Edit replacement book", position: 0 } })).json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, { data: {
    title: "Edit replacement", position: 0, kind: "chapter",
    content: { ops: [{ insert: `${original} ${identity}\nLater paragraph\n` }] },
  } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(editor.locator("p")).toHaveText([`${original} ${identity}`, "Later paragraph"]);
  await expectSaved(page);
  const before = (await (await request.get(`${providerUrl}/control`)).json()).length as number;
  if (!error) expect((await request.post(`${providerUrl}/control/arm`, { data: { identity, model: "e2e-edit" } })).status()).toBe(200);
  await editor.click();
  await editor.evaluate((element, length) => {
    const text = element.querySelector("p")?.firstChild;
    const next = element.querySelectorAll("p")[1];
    if (!text || !next) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    if (length === null) range.setEnd(next, 0);
    else range.setEnd(text, length);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  }, wholeParagraph ? null : original.length);
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
  await expect(note).toBeFocused();
  await note.fill("Fix the grammar only");
  if (error) await page.route(`${apiUrl}/ai/generate`, route => route.fulfill({
    contentType: "application/x-ndjson",
    body: `${JSON.stringify({ type: "error", error })}\n`,
  }));
  const generation = page.waitForResponse(response => response.url() === `${apiUrl}/ai/generate` && response.request().method() === "POST");
  await note.press("Enter");
  if (!error) {
    const recorded = await request.get(`${providerUrl}/control/${before}/request`);
    expect(recorded.status()).toBe(200);
    const upstream = await recorded.json();
    expect(upstream).toEqual({ id: before, identity, model: "e2e-edit", stream: true, closed: false, completed: false });
  }
  const controlUrl = `${providerUrl}/control/${before}`;
  const fragment = async (text: string) => expect((await request.post(`${controlUrl}/fragment`, { data: { text } })).status()).toBe(200);
  return { editor, failures, generation, fragment, chapterUrl: `${chaptersUrl}/${chapter.id}`, controlUrl };
}

async function expectSavedChapter(page: Page, request: APIRequestContext, chapterUrl: string, paragraphs: string[]) {
  await expect(page.locator(".ql-editor p")).toHaveText(paragraphs);
  await expectSaved(page);
  const saved = await request.get(chapterUrl);
  expect(saved.status()).toBe(200);
  expect((await saved.json()).content).toEqual({ ops: [{ insert: `${paragraphs.join("\n")}\n` }] });
}

test("Edit keeps only the replacement and drops commentary around it", async ({ page, request }) => {
  const identity = `GUZ865-commentary-${crypto.randomUUID()}`;
  const run = await openEditChapter(page, request, identity);
  await run.fragment("Here is the fix: <repla");
  await run.fragment("cement>He tried to dis");
  expect((await run.generation).status()).toBe(200);
  await expect(run.editor.locator("p").first()).toHaveText(`He tried to dis ${identity}`);
  await run.fragment("tract himself.</replac");
  await run.fragment("ement> This fixes the subject–noun agreement.\n\nShe opened the door and the story went on.");
  expect((await request.post(`${run.controlUrl}/finish`)).status()).toBe(200);
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
  await expectSavedChapter(page, request, run.chapterUrl, [`${replacement} ${identity}`, "Later paragraph"]);
  await expect(page.getByText("This fixes", { exact: false })).toHaveCount(0);
  expect(run.failures).toEqual([]);
});

for (const fragment of ["Revised paragraph", "Revised paragraph\n"]) {
  test(`Edit of a whole paragraph with its line ending and ${JSON.stringify(fragment)} keeps its paragraph break`, async ({ page, request }) => {
    const identity = `GUZ985-edit-${crypto.randomUUID()}`;
    const run = await openEditChapter(page, request, identity, true);
    await run.fragment(`<replacement>${fragment}</replacement>`);
    expect((await request.post(`${run.controlUrl}/finish`)).status()).toBe(200);
    expect((await run.generation).status()).toBe(200);
    await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
    await expectSavedChapter(page, request, run.chapterUrl, ["Revised paragraph", "Later paragraph"]);
    expect(run.failures).toEqual([]);
  });
}

for (const scenario of [
  { name: "missing", fragments: [replacement, " This fixes the agreement."], message: "AI edit response did not include a <replacement> tag." },
  { name: "unclosed", fragments: ["<replacement>He tried", " to distract"], message: "AI edit response ended before the closing </replacement> tag." },
  { name: "unclosed", fragments: ["<replacement>He tried", " to distract"], message: "AI edit response ended before the closing </replacement> tag.", wholeParagraph: true },
]) {
  test(`Edit with a ${scenario.name} replacement tag restores the ${scenario.wholeParagraph ? "whole paragraph" : "selection"}`, async ({ page, request }) => {
    const identity = `GUZ865-${scenario.name}-${crypto.randomUUID()}`;
    const run = await openEditChapter(page, request, identity, scenario.wholeParagraph);
    for (const text of scenario.fragments) await run.fragment(text);
    if (scenario.name === "unclosed") {
      await expect(run.editor.locator("p").first()).toHaveText(scenario.wholeParagraph ? "He tried to distract" : `He tried to distract ${identity}`);
    }
    expect((await request.post(`${run.controlUrl}/finish`)).status()).toBe(200);
    expect((await run.generation).status()).toBe(200);
    const error = page.getByText("The model didn't return an edit. Try again or switch models.", { exact: true });
    await expect(error).toBeVisible();
    await expect(error).toHaveAttribute("title", scenario.message);
    await expectSavedChapter(page, request, run.chapterUrl, [`${original} ${identity}`, "Later paragraph"]);
    expect(run.failures).toEqual([]);
  });
}

test("Edit shows other errors unchanged", async ({ page, request }) => {
  const identity = `GUZ921-other-error-${crypto.randomUUID()}`;
  const message = "Mock stream failure";
  const run = await openEditChapter(page, request, identity, false, message);
  expect((await run.generation).status()).toBe(200);
  await expect(page.getByText(message, { exact: true })).toBeVisible();
  await expectSavedChapter(page, request, run.chapterUrl, [`${original} ${identity}`, "Later paragraph"]);
  expect(run.failures).toEqual([]);
});
