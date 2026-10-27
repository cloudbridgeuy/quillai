import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

type Op = { insert?: unknown; retain?: number; delete?: number; attributes?: Record<string, unknown> };
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

async function surface(page: Page) {
  return page.evaluate(() => {
    const quill = (window as unknown as SpecWindow).specQuill;
    return { text: quill.getText(), length: quill.getLength(), ops: quill.getContents().ops as Op[], selection: quill.getSelection() };
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
      expect(text).toBe("\n");
      breaks.push(op.attributes["scene-break"]);
      scenes.push("");
    } else {
      scenes[scenes.length - 1] += text;
    }
  }
  return { breaks, scenes };
}

test("a script group reads as one scroll and syncs per scene", async ({ page, request, browserName }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const failures: string[] = [];
  const posts: string[] = [];
  const perSceneGets: string[] = [];
  page.on("pageerror", (error) => failures.push(error.message));
  page.on("response", (response) => {
    const url = response.url();
    if (!url.startsWith(api)) return;
    if (response.status() >= 400) failures.push(`${response.status()} ${url}`);
    const path = new URL(url).pathname;
    if (/^\/pieces\/\d+\/deltas$/.test(path)) {
      if (response.request().method() === "POST") posts.push(path);
      else perSceneGets.push(path);
    }
  });
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  const piece = async (id: number) => (await request.get(`${api}/pieces/${id}`)).json();
  const serverText = async (id: number) => (await piece(id)).content.ops.map((op: Op) => op.insert).join("");

  const universe = await post(`${api}/universes`, { title: `Script universe ${browserName}`, position: 0 });
  const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Garden script", position: 0, label: "screenplay", shape: "group" });
  const one = await post(`${api}/pieces/${group.id}/children`, { title: sceneOne, position: 0, label: "scene" });
  const two = await post(`${api}/pieces/${group.id}/children`, { title: sceneTwo, position: 1, label: "scene" });
  await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Kettle boils.\n" }] } }, 200);
  await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Rain falls.\n" }] } }, 200);

  const editor = page.locator(".ql-editor");
  const breaks = editor.locator(".ql-scene-break");
  const status = page.getByRole("status", { name: "Chapter save status", exact: true });
  const query = new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) });
  const expectSurface = async (oneText: string, twoText: string) => {
    await expect.poll(async () => sceneSlices((await surface(page)).ops)).toEqual({ breaks: [one.id, two.id], scenes: [oneText, twoText] });
  };
  const polls = async (count: number) => {
    for (let index = 0; index < count; index++) {
      await page.waitForResponse((response) => response.url().includes(`/pieces/${group.id}/scenes/deltas`));
    }
  };
  const expectServer = async (oneText: string, twoText: string) => {
    await expect.poll(() => serverText(one.id)).toBe(oneText);
    await expect.poll(() => serverText(two.id)).toBe(twoText);
  };
  const breakSafeServer = async () => {
    for (const id of [one.id, two.id]) {
      const ops: Op[] = (await piece(id)).content.ops;
      expect(ops.some((op) => op.attributes?.["scene-break"] !== undefined)).toBe(false);
    }
  };

  await test.step("two scenes read as one scroll with both break titles", async () => {
    for (const width of [800, 1100]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/?${query}`);
      await expectEditorReady(editor);
      await attachQuill(page);
      await expect(breaks).toHaveCount(2);
      await expect(breaks.nth(0)).toHaveAttribute("data-title", sceneOne);
      await expect(breaks.nth(1)).toHaveAttribute("data-title", sceneTwo);
      await expect(breaks.nth(0)).toHaveAttribute("contenteditable", "false");
      await expect(breaks.nth(1)).toBeVisible();
      expect(await breaks.nth(0).evaluate((node) => getComputedStyle(node, "::before").content)).toBe(`"${sceneOne}"`);
      await expect(editor).toContainText("Kettle boils.");
      await expect(editor).toContainText("Rain falls.");
      const view = await surface(page);
      expect(view.text).toBe("\nKettle boils.\n\nRain falls.\n");
      expect(view.length).toBe(view.text.length);
      await expect(status).toHaveText("Saved");
    }
    expect(posts).toEqual([]);
  });

  const twoStart = async () => 2 + sceneSlices((await surface(page)).ops).scenes[0]!.length;
  const oneEnd = async () => sceneSlices((await surface(page)).ops).scenes[0]!.length;

  await test.step("an edit in scene 2 saves only to scene 2", async () => {
    const before = await piece(one.id);
    await select(page, await twoStart() + "Rain falls".length);
    const saved = page.waitForResponse((response) => response.url().endsWith(`/pieces/${two.id}/deltas`) && response.request().method() === "POST");
    await page.keyboard.type(" hard");
    expect((await saved).status()).toBe(200);
    await expectServer("Kettle boils.\n", "Rain falls hard.\n");
    expect((await piece(one.id)).headRev).toBe(before.headRev);
    expect(posts.every((path) => path === `/pieces/${two.id}/deltas`)).toBe(true);
    await expect(status).toHaveText("Saved");
  });

  await test.step("markdown shortcuts stay off in a script", async () => {
    await select(page, await twoStart());
    await page.keyboard.type("# ");
    await expectServer("Kettle boils.\n", "# Rain falls hard.\n");
    expect((await surface(page)).ops.some((op) => op.attributes?.header !== undefined)).toBe(false);
    await page.keyboard.press("Backspace");
    await page.keyboard.press("Backspace");
    await expectServer("Kettle boils.\n", "Rain falls hard.\n");
  });

  await test.step("a remote edit in scene 1 keeps scene 2 text and the caret", async () => {
    const caret = await twoStart() + "Rain ".length;
    await select(page, caret);
    const head = (await piece(one.id)).headRev;
    await post(`${api}/pieces/${one.id}/deltas`, { baseRev: head, delta: { ops: [{ insert: "Steam. " }] } }, 200);
    await expectSurface("Steam. Kettle boils.\n", "Rain falls hard.\n");
    const view = await surface(page);
    expect(view.selection).toEqual({ index: caret + "Steam. ".length, length: 0 });
    expect(view.text.slice(view.selection!.index, view.selection!.index + 5)).toBe("falls");
    await expectServer("Steam. Kettle boils.\n", "Rain falls hard.\n");
  });

  await test.step("paste next to the break lands in one scene; paste across it is reverted", async () => {
    const paste = (text: string) => page.evaluate((text) => {
      const event = new ClipboardEvent("paste", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
      event.clipboardData!.setData("text/plain", text);
      (window as unknown as SpecWindow).specQuill.root.dispatchEvent(event);
    }, text);
    await select(page, await twoStart());
    await paste("Thunder. ");
    await expectSurface("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
    await expectServer("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
    const count = posts.length;
    await select(page, (await oneEnd()) - "boils.".length, "boils.\n\nThunder.".length);
    await paste("Merged");
    await expectSurface("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
    await expect(status).toHaveText("Edits across a scene break are not saved");
    await polls(2);
    expect(posts.length).toBe(count);
    await expectServer("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
  });

  await test.step("text dropped into the break line is reverted; a mouse drag keeps both scenes", async () => {
    const count = posts.length;
    await breaks.nth(1).evaluate((node) => node.insertBefore(document.createTextNode("Dropped"), node.firstChild));
    await expectSurface("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
    await expect(status).toHaveText("Edits across a scene break are not saved");
    await polls(2);
    expect(posts.length).toBe(count);

    await select(page, 1, "Steam.".length);
    const word = await editor.evaluate(() => window.getSelection()!.getRangeAt(0).getBoundingClientRect());
    const target = await breaks.nth(1).boundingBox();
    await page.mouse.move(word.x + word.width / 2, word.y + word.height / 2);
    await page.mouse.down();
    await page.mouse.move(target!.x + target!.width / 2, target!.y + target!.height / 2, { steps: 8 });
    await page.mouse.up();
    const after = sceneSlices((await surface(page)).ops);
    expect(after.breaks).toEqual([one.id, two.id]);
    await expect.poll(() => serverText(one.id)).toBe(sceneSlices((await surface(page)).ops).scenes[0]);
    await expect.poll(() => serverText(two.id)).toBe(sceneSlices((await surface(page)).ops).scenes[1]);
    await breakSafeServer();
    console.log(JSON.stringify({ browserName, drag: after.scenes }));
    const [oneText, twoText] = sceneSlices((await surface(page)).ops).scenes;
    for (const [id, text, want] of [[one.id, oneText!, "Steam. Kettle boils.\n"], [two.id, twoText!, "Thunder. Rain falls hard.\n"]] as const) {
      if (text === want) continue;
      const head = (await piece(id)).headRev;
      await post(`${api}/pieces/${id}/deltas`, { baseRev: head, delta: { ops: [{ delete: text.length - 1 }, { insert: want.slice(0, -1) }] } }, 200);
    }
    await expectSurface("Steam. Kettle boils.\n", "Thunder. Rain falls hard.\n");
  });

  await test.step("IME composition next to the break lands in the adjacent scene", async () => {
    const compose = async (text: string) => {
      if (browserName === "chromium") {
        const cdp = await page.context().newCDPSession(page);
        await cdp.send("Input.imeSetComposition", { text: text.slice(0, 1), selectionStart: 1, selectionEnd: 1 });
        await cdp.send("Input.imeSetComposition", { text, selectionStart: text.length, selectionEnd: text.length });
        await cdp.send("Input.insertText", { text });
        await cdp.detach();
      } else {
        await editor.evaluate((node, text) => {
          node.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "" }));
          node.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data: text }));
        }, text);
        await page.keyboard.insertText(text);
        await editor.evaluate((node, text) => {
          node.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: text }));
        }, text);
      }
    };
    await select(page, await twoStart());
    await compose("雨");
    await expectSurface("Steam. Kettle boils.\n", "雨Thunder. Rain falls hard.\n");
    await select(page, (await oneEnd()));
    await compose("湯");
    await expectSurface("Steam. Kettle boils.湯\n", "雨Thunder. Rain falls hard.\n");
    await expectServer("Steam. Kettle boils.湯\n", "雨Thunder. Rain falls hard.\n");
  });

  await test.step("a server edit that deletes scene 1's trailing newline does not merge scenes", async () => {
    const head = (await piece(one.id)).headRev;
    const length = "Steam. Kettle boils.湯".length;
    await post(`${api}/pieces/${one.id}/deltas`, { baseRev: head, delta: { ops: [{ retain: length }, { delete: 1 }] } }, 200);
    await expect.poll(async () => (await piece(one.id)).headRev).toBeGreaterThan(head + 1);
    await expectSurface("Steam. Kettle boils.湯\n", "雨Thunder. Rain falls hard.\n");
    await expect(breaks).toHaveCount(2);
    await expectServer("Steam. Kettle boils.湯\n", "雨Thunder. Rain falls hard.\n");
  });

  await test.step("an edit reported as delete and reinsert of a scene's last newline is saved", async () => {
    const twoSaves = posts.filter((path) => path.endsWith(`/${two.id}/deltas`)).length;
    await page.evaluate(([start]) => {
      const quill = (window as unknown as SpecWindow).specQuill;
      const Delta = quill.getContents().constructor as unknown as new () => ReturnType<Quill["getContents"]>;
      quill.updateContents(new Delta().retain(start!).delete("boils.湯\n".length).insert("whistles.\n"), "user");
    }, [1 + "Steam. Kettle ".length]);
    await expectSurface("Steam. Kettle whistles.\n", "雨Thunder. Rain falls hard.\n");
    await expectServer("Steam. Kettle whistles.\n", "雨Thunder. Rain falls hard.\n");
    expect(posts.filter((path) => path.endsWith(`/${two.id}/deltas`)).length).toBe(twoSaves);
    await expect(status).toHaveText("Saved");
  });

  await test.step("a failed scene save shows Not saved with the scene title and recovers", async () => {
    let held = true;
    await page.route(`${api}/pieces/${two.id}/deltas`, (route) => held && route.request().method() === "POST"
      ? route.abort("failed") : route.fallback());
    await select(page, await twoStart());
    await page.keyboard.type("Wind. ");
    await expect(status).toHaveText(/^Not saved/);
    await expect(status).toHaveAttribute("title", new RegExp(sceneTwo));
    held = false;
    await expect(status).toHaveText("Saved");
    await expectServer("Steam. Kettle whistles.\n", "Wind. 雨Thunder. Rain falls hard.\n");
    await page.unroute(`${api}/pieces/${two.id}/deltas`);
  });

  await test.step("reload and Resume open the group surface", async () => {
    await page.reload();
    await expectEditorReady(editor);
    await attachQuill(page);
    await expectSurface("Steam. Kettle whistles.\n", "Wind. 雨Thunder. Rain falls hard.\n");
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const resume = page.getByRole("region", { name: "Resume", exact: true });
    await expect(resume.getByText("Garden script", { exact: true })).toBeVisible();
    await resume.getByRole("button", { name: "Open", exact: true }).click();
    await expectEditorReady(editor);
    await attachQuill(page);
    expect(new URL(page.url()).searchParams.get("pieceId")).toBe(String(group.id));
    await expect(breaks).toHaveCount(2);
    await expectSurface("Steam. Kettle whistles.\n", "Wind. 雨Thunder. Rain falls hard.\n");
  });

  expect(perSceneGets).toEqual([]);
  await breakSafeServer();
  expect(failures).toEqual([]);
});
