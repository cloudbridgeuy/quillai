import { expect, test, type APIRequestContext, type Locator } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

async function post(request: APIRequestContext, url: string, data: object, status = 201) {
  const response = await request.post(url, { data });
  expect(response.status(), await response.text()).toBe(status);
  return response.json();
}

async function chapter(request: APIRequestContext, api: string, text = "Opening line\n") {
  const series = await post(request, `${api}/series`, { title: "Notice series", position: 0 });
  const book = await post(request, `${api}/series/${series.id}/books`, { title: "Notice book", position: 0 });
  const chapter = await post(request, `${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Notice chapter", position: 0, kind: "chapter", content: { ops: [{ insert: text }] },
  });
  return new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) });
}

async function notice(locator: Locator, tone: string, role: string) {
  await expect(locator).toBeVisible();
  await expect(locator).toHaveAttribute("data-tone", tone);
  await expect(locator).toHaveAttribute("role", role);
}

test.afterEach(async ({ request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (api) expect((await request.put(`${api}/settings/author-guidance`, { data: { text: "" } })).status()).toBe(200);
});

for (const width of [1100, 1440]) {
  test(`scene break warning at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const universe = await post(request, `${api}/universes`, { title: `Notice script ${width}`, position: 0 });
    const group = await post(request, `${api}/universes/${universe.id}/pieces`, { title: "Script", position: 0, label: "screenplay", shape: "group" });
    const one = await post(request, `${api}/pieces/${group.id}/children`, { title: "First scene", position: 0, label: "scene" });
    const two = await post(request, `${api}/pieces/${group.id}/children`, { title: "Second scene", position: 1, label: "scene" });
    await post(request, `${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "First line\n" }] } }, 200);
    await post(request, `${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Second line\n" }] } }, 200);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    await expect(editor.locator(".ql-scene-break")).toHaveCount(2);
    await editor.evaluate((element) => {
      type Fiber = { ref?: { current?: Quill }; child?: Fiber; sibling?: Fiber };
      for (let ancestor: Element | null = element; ancestor; ancestor = ancestor.parentElement) {
        const key = Object.keys(ancestor).find(name => name.startsWith("__reactFiber$"));
        const stack = key ? [(ancestor as unknown as Record<string, Fiber>)[key]!] : [];
        for (let visited = 0; stack.length && visited < 500; visited++) {
          const fiber = stack.pop()!;
          if (fiber.ref?.current?.root === element) {
            fiber.ref.current.focus();
            fiber.ref.current.setSelection("First line\n".length, 0, "user");
            return;
          }
          if (fiber.child) stack.push(fiber.child);
          if (fiber.sibling) stack.push(fiber.sibling);
        }
      }
      throw new Error("Mounted Quill instance not found");
    });
    await page.keyboard.press("Delete");
    await notice(page.locator(".editor-scene-notice").filter({ hasText: "Edits across a scene break are not saved" }), "warning", "status");
  });

  test(`empty Continue warning at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
    if (!provider) throw new Error("Run with cargo xtask e2e and its default catalog");
    const identity = `GUZ866-stop-${crypto.randomUUID()}`;
    const query = await chapter(request, api, `Opening ${identity}\n`);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${query}`);
    await expectEditorReady(page.locator(".ql-editor"));
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
    await page.getByRole("button", { name: /^Continue writing/ }).click();
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identity);
    expect((await request.post(`${provider}/control/${upstream.id}/finish`, { data: { reason: "stop" } })).status()).toBe(200);
    await notice(page.locator(".editor-scene-notice").filter({ hasText: "The model returned no text." }), "warning", "status");
  });

  test(`author guidance success at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    expect((await request.put(`${api}/settings/author-guidance`, { data: { text: "" } })).status()).toBe(200);
    const query = await chapter(request, api);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${query}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const drawer = page.locator("#context-drawer");
    await drawer.getByRole("textbox", { name: "Author guidance", exact: true }).fill("Keep the tone hopeful.");
    await drawer.getByRole("button", { name: "Save author guidance", exact: true }).click();
    await notice(drawer.getByText("Author guidance saved.", { exact: true }), "success", "status");
  });

  test(`tree deletion success at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const title = `Notice ${width} ${crypto.randomUUID()}`;
    const universe = await post(request, `${api}/universes`, { title: `${title} universe`, position: 0 });
    const piece = await post(request, `${api}/universes/${universe.id}/pieces`, { title: `${title} piece`, position: 0, label: "short_story", shape: "leaf" });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const tree = page.getByRole("region", { name: "Library", exact: true });
    const summary = tree.locator("summary").filter({ hasText: universe.title });
    if (await summary.locator("..").getAttribute("open") === null) await summary.click();
    const row = tree.getByRole("button", { name: piece.title, exact: true }).locator("xpath=ancestor::li[1]");
    await row.hover();
    await row.getByRole("button", { name: `Delete '${piece.title}'`, includeHidden: true }).click();
    const undo = tree.getByRole("button", { name: "Undo", exact: true });
    await expect(undo).toBeVisible();
    const deleted = undo.locator("..");
    await expect(deleted).toContainText(`Deleted '${piece.title}'.`);
    await notice(deleted, "success", "status");
  });

  for (const succeeds of [true, false]) {
    test(`full prompt copy ${succeeds ? "success" : "error"} at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      await page.addInitScript((succeeds) => {
        Object.defineProperty(navigator, "clipboard", { value: { writeText: async () => {
          if (!succeeds) throw new Error("Clipboard unavailable");
        } }, configurable: true });
      }, succeeds);
      const query = await chapter(request, api);
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(`/?${query}`);
      await expectEditorReady(page.locator(".ql-editor"));
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      await page.locator("#context-drawer").getByRole("button", { name: "Full prompt", exact: true }).click();
      const modal = page.getByRole("dialog", { name: "Full prompt", exact: true });
      await modal.getByRole("button", { name: "Copy", exact: true }).click();
      await notice(modal.getByText(succeeds ? "Copied" : "Could not copy prompt.", { exact: true }), succeeds ? "success" : "error", succeeds ? "status" : "alert");
    });
  }
}
