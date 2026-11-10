import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const price = { input_price_per_million: 0, output_price_per_million: 0 };
const catalog = (dialogue: string | null) => ({ models: ["text-a", "dialogue-model"].map(id => ({ id, label: id, ...price })), defaults: { generate: "text-a", edit: "text-a", dialogue: dialogue ?? "text-a" } });

for (const width of [1100, 1440]) {
  test(`Settings Dialogue model persists into defaults.dialogue at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    let saved: string | null = null;
    const writes: Record<string, string>[] = [];
    const defaults: unknown[] = [];
    await page.route("**/ai/**", route => route.abort());
    await page.route(`${api}/ai/models`, async route => {
      const body = catalog(saved);
      defaults.push(body.defaults.dialogue);
      await route.fulfill({ json: body });
    });
    await page.route(`${api}/settings/ai`, async route => {
      if (route.request().method() === "PUT") {
        const body = route.request().postDataJSON() as Record<string, string>;
        writes.push(body);
        saved = body.dialogue_model || null;
      }
      await route.fulfill({ json: { base_url: "https://env.example/v1", generate_model: null, edit_model: null, dialogue_model: saved, key_set: true, sources: { base_url: "env", api_key: "env", generate_model: "none", edit_model: "none", dialogue_model: saved ? "saved" : "none" } } });
    });
    const series = await (await request.post(`${api}/series`, { data: { title: `Dialogue settings ${width}`, position: 0 } })).json();
    const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
    const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, { data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening\n" }] } } })).json();
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings" });
    const menu = settings.getByRole("combobox", { name: "Dialogue model" });
    await expect(menu.locator("option").first()).toHaveText("Default");
    await expect(menu).toHaveValue("");
    await menu.selectOption("dialogue-model");
    await settings.getByRole("button", { name: "Save AI settings" }).click();
    await expect.poll(() => writes.length).toBe(1);
    expect(writes[0]).toEqual({ dialogue_model: "dialogue-model" });
    await expect.poll(() => defaults.at(-1)).toBe("dialogue-model");
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(settings.getByRole("combobox", { name: "Dialogue model" })).toHaveValue("dialogue-model");
    await expect(settings.locator("[data-model-action='dialogue']")).toContainText("Saved here");
  });
}
