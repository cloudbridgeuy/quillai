import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("AI settings save without exposing or resending the key and refresh models", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  let settings = { base_url: "https://env.example/v1" as string | null, generate_model: null as string | null, edit_model: null as string | null, key_set: true, sources: { base_url: "env", api_key: "env", generate_model: "none", edit_model: "none" } };
  const writes: Record<string, string>[] = [];
  await page.route(`${api}/settings/ai`, async route => {
    if (route.request().method() === "PUT") {
      const body = route.request().postDataJSON() as Record<string, string>;
      writes.push(body);
      if (body.base_url === "bad") return route.fulfill({ status: 422, json: { error: "Invalid URL", code: "VALIDATION_ERROR" } });
      settings = {
        base_url: body.base_url === undefined ? settings.base_url : body.base_url || "https://env.example/v1",
        generate_model: body.generate_model === undefined ? settings.generate_model : body.generate_model || null,
        edit_model: body.edit_model === undefined ? settings.edit_model : body.edit_model || null,
        key_set: true,
        sources: {
          base_url: body.base_url === undefined ? settings.sources.base_url : body.base_url ? "saved" : "env",
          api_key: body.api_key === undefined ? settings.sources.api_key : body.api_key ? "saved" : "env",
          generate_model: body.generate_model === undefined ? settings.sources.generate_model : body.generate_model ? "saved" : "none",
          edit_model: body.edit_model === undefined ? settings.sources.edit_model : body.edit_model ? "saved" : "none",
        },
      };
    }
    await route.fulfill({ json: settings });
  });
  const price = { input_price_per_million: 0, output_price_per_million: 0 };
  await page.route(`${api}/ai/models`, route => route.fulfill(settings.generate_model ? { json: { models: ["saved-model", "next-model"].map(id => ({ id, label: id, ...price })), defaults: { generate: settings.generate_model, edit: settings.edit_model ?? settings.generate_model } } } : { status: 503, json: { error: "missing", code: "AI_NOT_CONFIGURED" } }));
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
  await expect(dialog.getByRole("heading", { name: "AI", level: 3 })).toBeVisible();
  await expect(dialog.getByLabel("Base URL", { exact: true })).toBeVisible();
  await expect(dialog.getByText("Set in quillai.env. Edit that file to change it.")).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Remove", exact: true })).toHaveCount(0);
  await dialog.getByLabel("Base URL", { exact: true }).fill("bad");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect(dialog.getByRole("alert")).toContainText("Some of that doesn't look right. Check it and try again.");
  await dialog.getByLabel("Base URL", { exact: true }).fill("https://example.com/v1");
  // No catalog yet (AI not configured): one text field per action.
  await expect(dialog.getByLabel("Edit model", { exact: true })).toBeVisible();
  await dialog.getByLabel("Continue model", { exact: true }).fill("saved-model");
  await dialog.getByLabel("Edit model", { exact: true }).fill("saved-model");
  await dialog.getByRole("button", { name: "Replace", exact: true }).click();
  await dialog.getByLabel("Replacement API key").fill("secret-key-value");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect(dialog.getByText("Key saved · Saved here")).toBeVisible();
  await expect(dialog.getByLabel("Replacement API key")).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("secret-key-value");
  expect(await page.content()).not.toContain("secret-key-value");
  expect(writes[1]).toEqual({ base_url: "https://example.com/v1", generate_model: "saved-model", edit_model: "saved-model", api_key: "secret-key-value" });
  // The catalog is fetched again after the save; menus replace the text fields.
  await expect(dialog.getByRole("combobox", { name: "Continue model" })).toHaveValue("saved-model");
  await dialog.getByRole("combobox", { name: "Edit model" }).selectOption("next-model");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(3);
  expect(writes[2]).toEqual({ edit_model: "next-model" });
  await dialog.getByRole("combobox", { name: "Edit model" }).selectOption("");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(4);
  expect(writes[3]).toEqual({ edit_model: "" });
  await dialog.getByRole("combobox", { name: "Continue model" }).selectOption("next-model");
  await dialog.getByRole("combobox", { name: "Edit model" }).selectOption("next-model");
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(5);
  await dialog.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(dialog.getByText("The saved key will be removed on save. The quillai.env key, if present, will be used instead.")).toBeVisible();
  await dialog.getByRole("button", { name: "Save AI settings" }).click();
  await expect.poll(() => writes.length).toBe(6);
  expect(writes[5]).toEqual({ api_key: "" });
  await expect(dialog.getByText("Key saved · From quillai.env")).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Remove", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  // The Inspector has no model menus; its Model line shows the model saved in Settings.
  await expect(page.getByRole("combobox", { name: "Continue model" })).toHaveCount(0);
  const cost = page.getByRole("region", { name: "Model and cost", exact: true });
  await cost.scrollIntoViewIfNeeded();
  await expect(cost.locator("dt").filter({ hasText: /^Model$/ }).locator("+ dd")).toHaveText("next-model");
});
