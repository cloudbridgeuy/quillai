import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("Continue and Edit show their model and open Settings on the matching menu", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run with cargo xtask e2e");
  const models = [
    { id: "provider/fast", label: "Fast", input_price_per_million: 0.5, output_price_per_million: 1 },
    { id: "provider/precise", label: "Precise", input_price_per_million: 2, output_price_per_million: 6 },
  ];
  await page.route(`${apiUrl}/ai/models`, (route) => route.fulfill({
    json: { models, defaults: { generate: models[1]!.id, edit: models[0]!.id } },
  }));
  await page.route(`${apiUrl}/settings/ai`, (route) => route.fulfill({ json: {
    base_url: "https://env.example/v1", generate_model: models[1]!.id, edit_model: models[0]!.id, key_set: true,
    sources: { base_url: "env", api_key: "env", generate_model: "saved", edit_model: "saved" } } }));
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Label series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Label book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
    data: { title: "Label chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
  });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  const settings = page.getByRole("dialog", { name: "Settings", exact: true });
  const continueButton = page.getByRole("button", { name: "Continue", exact: true });

  await expect(continueButton).toHaveAttribute("title", "Continue (Ctrl/Cmd+Enter) · Precise");

  await editor.click();
  const continueLink = page.locator(".editor-continue-window-model");
  await expect(continueLink).toHaveText("Model: Precise");
  await expect(continueLink).toHaveAttribute("title", models[1]!.id);
  await continueLink.click();
  await expect(settings).toBeVisible();
  await expect(settings.getByRole("combobox", { name: "Continue model", exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(settings).toHaveCount(0);

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
  const editBox = page.getByRole("dialog", { name: "Edit instruction", exact: true });
  const editLink = editBox.getByRole("button", { name: "Model: Fast", exact: true });
  await expect(editLink).toHaveAttribute("title", models[0]!.id);
  await editBox.getByRole("textbox", { name: "Edit instruction", exact: true }).fill("Make it shorter");
  await editLink.click();
  await expect(settings.getByRole("combobox", { name: "Edit model", exact: true })).toBeFocused();
  // The typed instruction survives the trip to Settings.
  await page.getByRole("button", { name: "Edit selection", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Edit instruction", exact: true })).toHaveValue("Make it shorter");
});
