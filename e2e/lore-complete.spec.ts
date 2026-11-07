import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const visibleLabel = "Visible text (AI context)";
const hiddenLabel = "Hidden notes — author-only, never sent to AI";
const line = (event: object) => `${JSON.stringify(event)}\n`;
const delta = (text: string) => line({ type: "delta", delta: { ops: [{ insert: text }] } });
const done = line({ type: "done", finishReason: "stop" });
const price = { input_price_per_million: 0, output_price_per_million: 0 };
const catalog = (loreComplete: string | null) => ({ models: ["text-a", "lore-model"].map(id => ({ id, label: id, ...price })), defaults: { generate: "text-a", edit: "text-a", lore_complete: loreComplete ?? "text-a" } });

type Reply = { status?: number; body: string };

async function open(page: Page, request: APIRequestContext, width: number, reply: Reply) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const bodies: Record<string, unknown>[] = [];
  await page.route("**/ai/**", route => route.abort());
  await page.route(`${api}/ai/models`, route => route.fulfill({ json: catalog(null) }));
  await page.route(`${api}/ai/generate`, async route => {
    bodies.push(route.request().postDataJSON() as Record<string, unknown>);
    await route.fulfill(reply.status ? { status: reply.status, json: { error: "AI is not configured", code: "AI_NOT_CONFIGURED" } } : { contentType: "application/x-ndjson", body: reply.body });
  });
  const series = await (await request.post(`${api}/series`, { data: { title: `Lore complete ${width}`, position: 0 } })).json();
  const base = `${api}/series/${series.id}`;
  const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening\n" }] } } })).json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const lore = page.getByRole("region", { name: "Series lore", exact: true });
  await lore.getByRole("button", { name: "New lore", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Create lore", exact: true });
  await dialog.getByLabel("Name", { exact: true }).fill("Mira");
  await dialog.getByLabel("Keywords (one per line)").fill("secret-keyword");
  await dialog.getByLabel("Aliases (one per line)").fill("secret-alias");
  await dialog.getByLabel(hiddenLabel).fill("secret-hidden");
  await dialog.getByLabel(visibleLabel).fill("Born here. After");
  await dialog.getByLabel(visibleLabel).evaluate((field: HTMLTextAreaElement) => field.setSelectionRange(10, 10));
  return { api, base, lore, dialog, bodies, visible: dialog.getByLabel(visibleLabel), complete: dialog.getByRole("button", { name: "Complete", exact: true }), stop: dialog.getByRole("button", { name: "Stop", exact: true }) };
}

async function holdStream(page: Page) {
  await page.evaluate(partial => {
    const original = window.fetch.bind(window);
    window.fetch = (input, init) => {
      if (!String(input).endsWith("/ai/generate") || !String(init?.body).includes("lore_complete")) return original(input, init);
      const stream = new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(partial));
          init?.signal?.addEventListener("abort", () => controller.error(new DOMException("Aborted", "AbortError")));
        },
      });
      return Promise.resolve(new Response(stream, { headers: { "Content-Type": "application/x-ndjson" } }));
    };
  }, delta("Partial"));
}

for (const width of [1100, 1440]) {
  test(`Complete streams at the caret and Save keeps it after reload at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: delta("Raised") + delta(" by wolves.") + done });
    await run.complete.click();
    await expect(run.visible).toHaveValue("Born here. Raised by wolves. After");
    await run.dialog.getByRole("button", { name: "Save lore", exact: true }).click();
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await run.lore.getByRole("button", { name: "Mira · character", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "Edit lore", exact: true }).getByLabel(visibleLabel)).toHaveValue("Born here. Raised by wolves. After");
  });

  test(`Complete sends only name, kind and text before the caret at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    await run.complete.click();
    await expect(run.complete).toBeEnabled();
    expect(run.bodies).toEqual([{ action: "lore_complete", loreComplete: { name: "Mira", kind: "character", text: "Born here." } }]);
  });

  test(`only the visible field has Complete and Ctrl/Cmd+Enter at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    await expect(run.complete).toHaveCount(1);
    for (const label of [hiddenLabel, "Keywords (one per line)", "Aliases (one per line)"]) {
      await run.dialog.getByLabel(label).focus();
      await page.keyboard.press("ControlOrMeta+Enter");
    }
    await expect(run.complete).toBeEnabled();
    expect(run.bodies).toEqual([]);
  });

  test(`Ctrl/Cmd+Enter starts and stops, and Stop removes only the AI text at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    await holdStream(page);
    await run.visible.focus();
    await run.visible.evaluate((field: HTMLTextAreaElement) => field.setSelectionRange(10, 10));
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(run.visible).toHaveValue("Born here. Partial After");
    await expect(run.stop).toBeVisible();
    await expect(run.visible).toHaveAttribute("readonly", "");
    await expect(run.dialog.getByRole("button", { name: "Save lore", exact: true })).toBeDisabled();
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(run.visible).toHaveValue("Born here. After");
    await expect(run.complete).toBeEnabled();
    await expect(run.dialog.getByRole("alert")).toHaveCount(0);
  });

  test(`a server error and an early end roll back and keep the draft at ${width}px`, async ({ page, request }) => {
    let reply = { body: delta("Lost") + line({ type: "error", error: "provider failed" }) };
    const run = await open(page, request, width, { get body() { return reply.body; } });
    await run.complete.click();
    await expect(run.dialog.getByRole("alert")).toContainText("provider failed");
    await expect(run.visible).toHaveValue("Born here. After");
    reply = { body: delta("Lost") };
    await run.visible.evaluate((field: HTMLTextAreaElement) => field.setSelectionRange(10, 10));
    await run.complete.click();
    await expect(run.dialog.getByRole("alert")).toContainText("The note could not be completed.");
    await expect(run.visible).toHaveValue("Born here. After");
    await expect(run.dialog.getByLabel("Name", { exact: true })).toHaveValue("Mira");
    await expect(run.dialog.getByLabel(hiddenLabel)).toHaveValue("secret-hidden");
  });

  test(`503 AI_NOT_CONFIGURED shows the setup error at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { status: 503, body: "" });
    await run.complete.click();
    await expect(run.dialog.getByRole("alert")).toContainText("Open Settings → AI");
    await expect(run.visible).toHaveValue("Born here. After");
  });

  test(`Escape during a stream cancels and keeps the saved entry at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    await run.dialog.getByRole("button", { name: "Save lore", exact: true }).click();
    const entry = run.lore.getByRole("button", { name: "Mira · character", exact: true });
    await entry.click();
    const edit = page.getByRole("dialog", { name: "Edit lore", exact: true });
    await holdStream(page);
    await edit.getByLabel(visibleLabel).evaluate((field: HTMLTextAreaElement) => field.setSelectionRange(10, 10));
    await edit.getByRole("button", { name: "Complete", exact: true }).click();
    await expect(edit.getByLabel(visibleLabel)).toHaveValue("Born here. Partial After");
    await page.keyboard.press("Escape");
    await expect(edit).toHaveCount(0);
    const saved = await (await request.get(`${run.base}/lore`)).json();
    expect(saved).toEqual([expect.objectContaining({ name: "Mira", visible_text: "Born here. After" })]);
  });

  test(`Settings saves the Lore completion model at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    let saved: string | null = null;
    const writes: Record<string, string>[] = [];
    await page.route(`${run.api}/ai/models`, route => route.fulfill({ json: catalog(saved) }));
    await page.route(`${run.api}/settings/ai`, async route => {
      if (route.request().method() === "PUT") {
        const body = route.request().postDataJSON() as Record<string, string>;
        writes.push(body);
        saved = body.lore_complete_model || null;
      }
      await route.fulfill({ json: { base_url: "https://env.example/v1", generate_model: null, edit_model: null, lore_complete_model: saved, key_set: true, sources: { base_url: "env", api_key: "env", generate_model: "none", edit_model: "none", lore_complete_model: saved ? "saved" : "none" } } });
    });
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings" });
    const menu = settings.getByRole("combobox", { name: "Lore completion model" });
    await expect(menu.locator("option").first()).toHaveText("Default");
    await menu.selectOption("lore-model");
    await settings.getByRole("button", { name: "Save AI settings" }).click();
    await expect.poll(() => writes.length).toBe(1);
    expect(writes[0]).toEqual({ lore_complete_model: "lore-model" });
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(settings.getByRole("combobox", { name: "Lore completion model" })).toHaveValue("lore-model");
  });

  test(`Leave out hides Complete and the shortcut still runs Continue outside at ${width}px`, async ({ page, request }) => {
    const run = await open(page, request, width, { body: done });
    await run.dialog.getByLabel("In AI requests").selectOption("leave_out");
    await expect(run.complete).toHaveCount(0);
    await run.visible.focus();
    await page.keyboard.press("ControlOrMeta+Enter");
    expect(run.bodies).toEqual([]);
    await page.keyboard.press("Escape");
    await page.locator(".ql-editor").click();
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect.poll(() => run.bodies.map(body => body.action)).toEqual(["generate"]);
  });
}
