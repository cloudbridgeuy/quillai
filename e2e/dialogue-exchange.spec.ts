import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
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

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type DialogueMock = { mode: "done" | "hold" | "error" | "503"; chunks: string[]; requests: string[] };
declare global {
  interface Window { dialogueMock?: DialogueMock }
}

async function mockGenerate(page: Page) {
  await page.addInitScript(() => {
    const original = window.fetch;
    const mock: DialogueMock = { mode: "done", chunks: [], requests: [] };
    window.dialogueMock = mock;
    window.fetch = async (input, init) => {
      if (!String(input).endsWith("/ai/generate")) return original(input, init);
      mock.requests.push(String(init?.body));
      if (mock.mode === "503") return new Response(JSON.stringify({ error: "missing QUILLAI_AI_API_KEY", code: "AI_NOT_CONFIGURED" }), { status: 503, headers: { "Content-Type": "application/json" } });
      const encoder = new TextEncoder();
      const line = (event: object) => encoder.encode(`${JSON.stringify(event)}\n`);
      return new Response(new ReadableStream<Uint8Array>({
        start(controller) {
          for (const chunk of mock.chunks) controller.enqueue(line({ type: "delta", delta: { ops: [{ insert: chunk }] } }));
          if (mock.mode === "hold") {
            init?.signal?.addEventListener("abort", () => controller.error(new DOMException("Stopped", "AbortError")), { once: true });
            return;
          }
          controller.enqueue(line(mock.mode === "error" ? { type: "error", error: "Provider failed" } : { type: "done", finishReason: "stop" }));
          controller.close();
        },
      }), { headers: { "Content-Type": "application/x-ndjson" } });
    };
  });
}

const setMock = (page: Page, mode: DialogueMock["mode"], chunks: string[]) =>
  page.evaluate(([mode, chunks]) => Object.assign(window.dialogueMock!, { mode, chunks }), [mode, chunks] as const);
const sent = (page: Page) => page.evaluate(() => window.dialogueMock!.requests.map(body => JSON.parse(body) as Record<string, unknown>));
const lineStart = process.platform === "darwin" ? "Meta+ArrowLeft" : "Home";
const plain = (ops: Op[]) => ops.map(op => (typeof op.insert === "string" ? op.insert : "")).join("");

async function proseChapter(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  await mockGenerate(page);
  const series = await (await request.post(`${api}/series`, { data: { title: `Dialogue ${width}`, position: 0 } })).json();
  for (const [kind, name, level, pinned] of [["character", "Bram", "known_dont_reveal", false], ["character", "Ana", "include", true], ["character", "Ghost", "leave_out", true], ["place", "Keep", "include", false]] as const) {
    await request.post(`${api}/series/${series.id}/lore`, { data: { kind, name, visible_text: `${name} visible.`, hidden_notes: `${name} hidden.`, level, pinned } });
  }
  const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await (await request.post(chapters, { data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "They reached the Keep.\n" }] } } })).json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  const saved = async () => plain((await (await request.get(`${chapters}/${chapter.id}`)).json()).content.ops);
  return { editor, saved };
}

async function writeDialogue(page: Page, speakers: string[], brief = "", caret?: number) {
  await page.locator(".ql-editor").click();
  await page.keyboard.press("Control+End");
  if (caret !== undefined) {
    await page.keyboard.press(lineStart);
    for (let step = 0; step < caret; step++) await page.keyboard.press("ArrowRight");
  }
  await page.getByRole("button", { name: "Dialogue", exact: true }).click();
  const box = page.getByRole("dialog", { name: "Dialogue", exact: true });
  for (const name of speakers) await box.getByRole("checkbox", { name, exact: true }).check();
  if (brief) await box.getByRole("textbox", { name: "Dialogue brief", exact: true }).fill(brief);
  await box.getByRole("button", { name: "Write", exact: true }).click();
}

const exchange = ["\"We go now,\" Ana said.", "\n\n\"Not yet,\" said Bram."];
const written = "They reached the Keep.\n\"We go now,\" Ana said.\n\n\"Not yet,\" said Bram.\n";

for (const width of [1100, 1440]) {
  test(`two speakers stream at the caret and persist after reload at ${width}px`, async ({ page, request }) => {
    const { editor, saved } = await proseChapter(page, request, width);
    await setMock(page, "done", exchange);
    await writeDialogue(page, ["Ana", "Bram"]);
    await expect(editor.locator("p")).toHaveText(["They reached the Keep.", "\"We go now,\" Ana said.", "", "\"Not yet,\" said Bram."]);
    await expect.poll(saved).toBe(written);
    await page.reload();
    await expectEditorReady(editor);
    await expect.poll(saved).toBe(written);
    await expect(editor).toContainText("\"Not yet,\" said Bram.");
  });

  test(`a mid-line caret splits the line and the dialogue starts a new paragraph there at ${width}px`, async ({ page, request }) => {
    const { editor, saved } = await proseChapter(page, request, width);
    await setMock(page, "done", exchange);
    await writeDialogue(page, ["Ana"], "", "They reached".length);
    await expect(editor.locator("p")).toHaveText(["They reached", "\"We go now,\" Ana said.", "", "\"Not yet,\" said Bram.", "the Keep."]);
    await expect.poll(saved).toBe("They reached\n\"We go now,\" Ana said.\n\n\"Not yet,\" said Bram.\n the Keep.\n");
  });

  test(`the request holds speakers, brief and speaker lore first with no hidden notes or model at ${width}px`, async ({ page, request }) => {
    await proseChapter(page, request, width);
    await setMock(page, "done", []);
    await writeDialogue(page, ["Bram", "Ana"], "argue about the gate");
    await expect.poll(async () => (await sent(page)).length).toBe(1);
    const [body] = await sent(page);
    expect(body!.action).toBe("dialogue");
    expect(body!.dialogue).toEqual({ speakers: ["Bram", "Ana"], brief: "argue about the gate" });
    expect((body!.lore as { name: string }[]).map(entry => entry.name)).toEqual(["Bram", "Ana", "Keep"]);
    expect(body).not.toHaveProperty("model");
    expect(JSON.stringify(body)).not.toContain("hidden");
  });

  test(`a Leave out character is not listed at ${width}px`, async ({ page, request }) => {
    await proseChapter(page, request, width);
    await page.getByRole("button", { name: "Dialogue", exact: true }).click();
    const box = page.getByRole("dialog", { name: "Dialogue", exact: true });
    await expect(box.getByRole("checkbox")).toHaveCount(2);
    await expect(box.getByRole("checkbox", { name: "Ana", exact: true })).toBeVisible();
    await expect(box.getByRole("checkbox", { name: "Bram", exact: true })).toBeVisible();
    await expect(box).not.toContainText("Ghost");
    await expect(box).not.toContainText("Keep");
  });

  test(`Stop and a server error roll back only AI text and keep author inserts at ${width}px`, async ({ page, request }) => {
    const { editor, saved } = await proseChapter(page, request, width);
    await setMock(page, "hold", exchange);
    await writeDialogue(page, ["Ana"]);
    await expect(editor).toContainText("Not yet");
    await editor.locator("p").first().click();
    await page.keyboard.press(lineStart);
    await page.keyboard.type("Then ");
    await page.getByRole("button", { name: "Stop AI", exact: true }).click();
    await expect(editor.locator("p")).toHaveText(["Then They reached the Keep."]);
    await setMock(page, "error", exchange);
    await writeDialogue(page, ["Ana"]);
    await expect(page.locator(".editor-run-status")).toHaveText("Provider failed");
    await expect(editor.locator("p")).toHaveText(["Then They reached the Keep."]);
    await expect.poll(saved).toBe("Then They reached the Keep.\n");
  });

  test(`a screenplay group gets character and dialogue blocks at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    await mockGenerate(page);
    const post = async (url: string, data: object) => (await request.post(url, { data })).json();
    const universe = await post(`${api}/universes`, { title: `Dialogue script ${width}`, position: 0 });
    await post(`${api}/universes/${universe.id}/lore`, { kind: "character", name: "Ana", visible_text: "A pilot.", level: "include" });
    const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Script", position: 0, label: "screenplay", shape: "group" });
    const scene = await post(`${api}/pieces/${group.id}/children`, { title: "INT. HANGAR - DAY", position: 0, label: "scene" });
    await request.post(`${api}/pieces/${scene.id}/deltas`, { data: { baseRev: 0, delta: { ops: [{ insert: "Engines idle.\n" }] } } });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await setMock(page, "done", ["ANA\n(low)\nWe fly", " tonight.\n"]);
    await editor.getByText("Engines idle.").click();
    await page.keyboard.press("End");
    await page.getByRole("button", { name: "Dialogue", exact: true }).click();
    const box = page.getByRole("dialog", { name: "Dialogue", exact: true });
    await box.getByRole("checkbox", { name: "Ana", exact: true }).check();
    await box.getByRole("button", { name: "Write", exact: true }).click();
    const block = (type: string) => ({ "script-block": type });
    await expect.poll(async () => (await (await request.get(`${api}/pieces/${scene.id}`)).json()).content.ops).toEqual([
      { insert: "Engines idle.\nANA" }, { insert: "\n", attributes: block("character") },
      { insert: "(low)" }, { insert: "\n", attributes: block("parenthetical") },
      { insert: "We fly tonight." }, { insert: "\n", attributes: block("dialogue") },
      { insert: "\n" },
    ]);
  });

  test(`503 AI_NOT_CONFIGURED shows the error with no text change at ${width}px`, async ({ page, request }) => {
    const { editor, saved } = await proseChapter(page, request, width);
    await setMock(page, "503", []);
    await writeDialogue(page, ["Ana"]);
    await expect(page.locator(".editor-run-status")).toHaveText("AI isn't set up yet. Open Settings → AI to add your provider.");
    await expect(editor.locator("p")).toHaveText(["They reached the Keep."]);
    await expect.poll(saved).toBe("They reached the Keep.\n");
    await expect(page.locator("body")).not.toContainText("QUILLAI_AI_API_KEY");
  });
}
