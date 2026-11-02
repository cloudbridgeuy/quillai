import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type SpecWindow = { specQuill: Quill };

const sceneOne = "INT. KITCHEN - DAY";
const sceneTwo = "EXT. GARDEN - NIGHT";

async function attachQuill(page: Page) {
  await page.locator(".ql-editor").evaluate((element) => {
    type Fiber = { ref?: { current?: Quill }; child?: Fiber; sibling?: Fiber };
    for (let ancestor: Element | null = element; ancestor; ancestor = ancestor.parentElement) {
      const key = Object.keys(ancestor).find((name) => name.startsWith("__reactFiber$"));
      const stack = key ? [(ancestor as unknown as Record<string, Fiber>)[key]!] : [];
      for (let visited = 0; stack.length && visited < 500; visited++) {
        const fiber = stack.pop()!;
        if (fiber.ref?.current?.root === element) {
          (window as unknown as SpecWindow).specQuill = fiber.ref.current;
          return;
        }
        if (fiber.child) stack.push(fiber.child);
        if (fiber.sibling) stack.push(fiber.sibling);
      }
    }
    throw new Error("Mounted Quill instance not found");
  });
}

async function contents(page: Page) {
  return page.evaluate(() => {
    const quill = (window as unknown as SpecWindow).specQuill;
    return { ops: quill.getContents().ops as Op[], selection: quill.getSelection() };
  });
}

async function select(page: Page, index: number, length = 0) {
  await page.evaluate(([index, length]) => {
    const quill = (window as unknown as SpecWindow).specQuill;
    quill.focus();
    quill.setSelection(index!, length!, "user");
  }, [index, length]);
}

function sceneSlices(ops: Op[]): { breaks: unknown[]; scenes: string[] } {
  const breaks: unknown[] = [];
  const scenes: string[] = [];
  for (const op of ops) {
    const text = typeof op.insert === "string" ? op.insert : "";
    if (op.attributes?.["scene-break"] !== undefined) {
      breaks.push(op.attributes["scene-break"]);
      scenes.push("");
    } else {
      scenes[scenes.length - 1] += text;
    }
  }
  return { breaks, scenes };
}

test("the script surface moves across scene breaks and blocks merging edits", async ({ page, request, browserName }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const failures: string[] = [];
  const posts: string[] = [];
  page.on("pageerror", (error) => failures.push(error.message));
  page.on("response", (response) => {
    const url = response.url();
    if (!url.startsWith(api)) return;
    if (response.status() >= 400) failures.push(`${response.status()} ${url}`);
    if (response.request().method() === "POST" && /^\/pieces\/\d+\/deltas$/.test(new URL(url).pathname)) posts.push(url);
  });
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  const serverText = async (id: number) => (await (await request.get(`${api}/pieces/${id}`)).json()).content.ops.map((op: Op) => op.insert).join("");

  const universe = await post(`${api}/universes`, { title: `Cross scene ${browserName}`, position: 0 });
  const prose = await post(`${api}/universes/${universe.id}/pieces`, { title: "Prose note", position: 0, label: "short_story", shape: "leaf" });
  const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Garden script", position: 1, label: "screenplay", shape: "group" });
  const one = await post(`${api}/pieces/${group.id}/children`, { title: sceneOne, position: 0, label: "scene" });
  const two = await post(`${api}/pieces/${group.id}/children`, { title: sceneTwo, position: 1, label: "scene" });
  await post(`${api}/pieces/${prose.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Plain prose.\n" }] } }, 200);
  await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Kettle boils.\n" }] } }, 200);
  await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Rain falls.\n" }] } }, 200);

  const editor = page.locator(".ql-editor");
  const notice = page.getByRole("status").filter({ hasText: "scene break" });
  const drawer = page.locator("#manuscript-drawer");
  const expectBoth = async (oneText: string, twoText: string) => {
    await expect.poll(async () => sceneSlices((await contents(page)).ops)).toEqual({ breaks: [one.id, two.id], scenes: [oneText, twoText] });
    await expect.poll(() => serverText(one.id)).toBe(oneText);
    await expect.poll(() => serverText(two.id)).toBe(twoText);
  };
  const polls = async (count: number) => {
    for (let index = 0; index < count; index++) {
      await page.waitForResponse((response) => response.url().includes(`/pieces/${group.id}/scenes/deltas`));
    }
  };
  const twoStart = async () => 2 + sceneSlices((await contents(page)).ops).scenes[0]!.length;
  const oneEnd = async () => sceneSlices((await contents(page)).ops).scenes[0]!.length;
  const caret = async () => (await contents(page)).selection;
  const openInTree = async (name: string) => {
    if (!(await drawer.isVisible())) await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await drawer.getByRole("button", { name, exact: true }).click();
  };

  await page.setViewportSize({ width: 1100, height: 900 });

  await test.step("a scene in the tree opens the whole surface with the caret in that scene", async () => {
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(prose.id) })}`);
    await expectEditorReady(editor);
    await expect(editor).toHaveText("Plain prose.");
    await openInTree(sceneTwo);
    await expect(editor.locator(".ql-scene-break")).toHaveCount(2);
    await expectEditorReady(editor);
    await attachQuill(page);
    expect(new URL(page.url()).searchParams.get("pieceId")).toBe(String(group.id));
    await expect.poll(caret).toEqual({ index: await twoStart(), length: 0 });
    await page.keyboard.type("Wind. ");
    await expectBoth("Kettle boils.\n", "Wind. Rain falls.\n");
  });

  await test.step("the script group in the tree opens the surface with the caret in the first scene", async () => {
    await openInTree("Garden script");
    await expect.poll(() => page.evaluate(() => (window as unknown as Partial<SpecWindow>).specQuill)).toBeUndefined();
    await expectEditorReady(editor);
    await attachQuill(page);
    await expect.poll(caret).toEqual({ index: 1, length: 0 });
    await page.keyboard.type("Steam. ");
    await expectBoth("Steam. Kettle boils.\n", "Wind. Rain falls.\n");
  });

  await test.step("arrow keys step over the break line", async () => {
    await select(page, await oneEnd());
    await page.keyboard.press("ArrowRight");
    await expect.poll(caret).toEqual({ index: await twoStart(), length: 0 });
    await page.keyboard.press("ArrowLeft");
    await expect.poll(caret).toEqual({ index: await oneEnd(), length: 0 });
    await page.keyboard.press("ArrowDown");
    await expect.poll(async () => (await caret())!.index).toBeGreaterThanOrEqual(await twoStart());
    await page.keyboard.press("ArrowUp");
    await expect.poll(async () => (await caret())!.index).toBeLessThanOrEqual(await oneEnd());
    await select(page, await oneEnd());
    await page.keyboard.press("ArrowRight");
    await page.keyboard.type("Dusk. ");
    await expectBoth("Steam. Kettle boils.\n", "Dusk. Wind. Rain falls.\n");
  });

  await test.step("a selection spans scenes", async () => {
    await select(page, (await oneEnd()) - "boils.".length);
    for (let step = 0; step < "boils.".length + 3; step++) await page.keyboard.press("Shift+ArrowRight");
    const range = (await caret())!;
    expect(range.index).toBe((await oneEnd()) - "boils.".length);
    expect(range.index + range.length).toBeGreaterThan(await twoStart());
    await expect.poll(() => page.evaluate(() => window.getSelection()!.toString())).toMatch(/^boils\.\s+D/);
  });

  await test.step("delete, replace, and paste across a break are blocked with a notice", async () => {
    const count = posts.length;
    const across = async () => select(page, (await oneEnd()) - "boils.".length, "boils.\n\nDusk.".length);
    for (const key of ["Delete", "Backspace", "x", "Enter"]) {
      await across();
      await page.keyboard.press(key);
      await expect(notice).toBeVisible();
      await expectBoth("Steam. Kettle boils.\n", "Dusk. Wind. Rain falls.\n");
    }
    await across();
    await page.evaluate(() => {
      const event = new ClipboardEvent("paste", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
      event.clipboardData!.setData("text/plain", "Merged");
      (window as unknown as SpecWindow).specQuill.root.dispatchEvent(event);
    });
    await expect(notice).toBeVisible();
    for (const width of [1100, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await expect(notice).toBeInViewport();
      await page.screenshot({ path: `../target/guz-984/${test.info().project.name}/cross-scene-notice-${width}.png`, fullPage: true });
    }
    await page.setViewportSize({ width: 1100, height: 900 });
    await polls(2);
    expect(posts.length).toBe(count);
    await expectBoth("Steam. Kettle boils.\n", "Dusk. Wind. Rain falls.\n");
  });

  await test.step("Backspace at a scene start and Delete at a scene end do not merge scenes", async () => {
    await select(page, await twoStart());
    await page.keyboard.type("A");
    await expect(notice).toBeHidden();
    await expectBoth("Steam. Kettle boils.\n", "ADusk. Wind. Rain falls.\n");
    await page.keyboard.press("Backspace");
    await expectBoth("Steam. Kettle boils.\n", "Dusk. Wind. Rain falls.\n");
    await expect(notice).toBeHidden();
    const settled = posts.length;
    await page.keyboard.press("Backspace");
    await expect(notice).toBeVisible();
    await select(page, await oneEnd());
    await page.keyboard.press("Delete");
    await expect(notice).toBeVisible();
    await polls(2);
    expect(posts.length).toBe(settled);
    await expect(editor.locator(".ql-scene-break")).toHaveCount(2);
    await expectBoth("Steam. Kettle boils.\n", "Dusk. Wind. Rain falls.\n");
  });

  await test.step("Enter at a scene end stays in that scene", async () => {
    await select(page, await oneEnd());
    await page.keyboard.press("Enter");
    await page.keyboard.type("Later");
    await expect(notice).toBeHidden();
    await expectBoth("Steam. Kettle boils.\nLater\n", "Dusk. Wind. Rain falls.\n");
  });

  await test.step("a prose piece in the tree still opens alone", async () => {
    await openInTree("Prose note");
    await expectEditorReady(editor);
    await expect(editor).toHaveText("Plain prose.");
    await expect(editor.locator(".ql-scene-break")).toHaveCount(0);
    expect(new URL(page.url()).searchParams.get("pieceId")).toBe(String(prose.id));
    await page.reload();
    await expectEditorReady(editor);
    await attachQuill(page);
    await expect(editor).toHaveText("Plain prose.");
    await openInTree(sceneOne);
    await expect(editor.locator(".ql-scene-break")).toHaveCount(2);
    await expectEditorReady(editor);
    await attachQuill(page);
    await expect.poll(caret).toEqual({ index: 1, length: 0 });
  });

  await expectBoth("Steam. Kettle boils.\nLater\n", "Dusk. Wind. Rain falls.\n");
  expect(failures).toEqual([]);
});
