import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("AI settings save without exposing or resending the key and refresh models", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  let settings = { base_url: "https://env.example/v1" as string | null, model: null as string | null, key_set: false, sources: { base_url: "env", model: "none", api_key: "none" } };
  const writes: Record<string, string>[] = [];
  await page.route(`${api}/settings/ai`, async route => {
    if (route.request().method() === "PUT") {
      const body = route.request().postDataJSON() as Record<string, string>;
      writes.push(body);
      if (body.base_url === "bad") return route.fulfill({ status: 422, json: { error: "Invalid URL", code: "VALIDATION_ERROR" } });
      settings = {
        base_url: body.base_url === undefined ? settings.base_url : body.base_url || "https://env.example/v1",
        model: body.model === undefined ? settings.model : body.model || null,
        key_set: body.api_key === undefined ? settings.key_set : body.api_key !== "",
        sources: {
          base_url: body.base_url === undefined ? settings.sources.base_url : body.base_url ? "saved" : "env",
          model: body.model === undefined ? settings.sources.model : body.model ? "saved" : "none",
          api_key: body.api_key === undefined ? settings.sources.api_key : body.api_key ? "saved" : "none",
        },
      };
    }
    await route.fulfill({ json: settings });
  });
  await page.route(`${api}/ai/models`, route => route.fulfill(settings.model ? { json: { models: [{ id: settings.model, label: settings.model, input_price_per_million: 0, output_price_per_million: 0 }], defaults: { generate: settings.model, edit: settings.model } } } : { status: 503, json: { error: "missing", code: "AI_NOT_CONFIGURED" } }));
  const series = await (await request.post(`${api}/series`, { data: { title: "AI settings", position: 0 } })).json();
  const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, { data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening\n" }] } } })).json();
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await page.getByText("Advanced AI controls", { exact: true }).click();
  await expect(page.getByText("AI isn't set up yet. Open Settings → AI to add your provider.")).toBeVisible();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await dialog.getByText("AI", { exact: true }).click();
  await expect(dialog.getByText("From quillai.env", { exact: true })).toBeVisible();
  await dialog.getByLabel("Base URL", { exact: true }).fill("bad");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect(dialog.getByRole("alert")).toContainText("Invalid URL");
  await dialog.getByLabel("Base URL", { exact: true }).fill("https://example.com/v1");
  await dialog.getByLabel("Model", { exact: true }).fill("saved-model");
  await dialog.getByRole("button", { name: "Replace", exact: true }).click();
  await dialog.getByLabel("Replacement API key").fill("secret-key-value");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect(dialog.getByText("Key saved · Saved here")).toBeVisible();
  await expect(dialog.getByLabel("Replacement API key")).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("secret-key-value");
  expect(await page.content()).not.toContain("secret-key-value");
  expect(writes[1]).toEqual({ base_url: "https://example.com/v1", model: "saved-model", api_key: "secret-key-value" });
  await dialog.getByLabel("Model", { exact: true }).fill("next-model");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(3);
  expect(writes[2]).toEqual({ model: "next-model" });
  await dialog.getByRole("button", { name: "Remove", exact: true }).click();
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(4);
  expect(writes[3]).toEqual({ api_key: "" });
  await expect(dialog.getByText("No key set · Not set")).toBeVisible();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const advanced = page.locator(".editor-advanced-section");
  if (await advanced.getAttribute("open") === null) await advanced.locator("summary").click();
  await expect(page.getByRole("combobox", { name: "Continue model" })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Continue model" })).toHaveValue("next-model");
});
