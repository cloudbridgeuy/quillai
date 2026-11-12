import { expect, test, type Page, type Route } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type SpecWindow = { specQuill: Quill };

const acrossBreak = "Select text inside one scene to use Edit.";

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

async function sceneTexts(page: Page): Promise<string[]> {
  const ops = await page.evaluate(() => (window as unknown as SpecWindow).specQuill.getContents().ops as Op[]);
  const scenes: string[] = [];
  for (const op of ops) {
    if (op.attributes?.["scene-break"] !== undefined) scenes.push("");
    else scenes[scenes.length - 1] += typeof op.insert === "string" ? op.insert : "";
  }
  return scenes;
}

async function select(page: Page, index: number, length = 0) {
  await page.evaluate(([index, length]) => {
    const quill = (window as unknown as SpecWindow).specQuill;
    quill.focus();
    quill.setSelection(index!, length!, "user");
  }, [index, length]);
}

test("Continue and Edit on the script surface stay inside one scene at 1100px", async ({ page, request, browserName }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const failures: string[] = [];
  const posts: number[] = [];
  let aiRunning = false;
  let pollsWhileAi = 0;
  page.on("pageerror", (error) => failures.push(error.message));
  page.on("request", (sent) => {
    if (aiRunning && sent.url().includes("/scenes/deltas")) pollsWhileAi++;
  });
  page.on("response", (response) => {
    const url = response.url();
    if (!url.startsWith(api)) return;
    if (response.status() >= 400) failures.push(`${response.status()} ${url}`);
    const posted = /^\/pieces\/(\d+)\/deltas$/.exec(new URL(url).pathname);
    if (posted && response.request().method() === "POST") posts.push(Number(posted[1]));
  });
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  const server = async (id: number) => (await (await request.get(`${api}/pieces/${id}`)).json()) as { content: { ops: Op[] }; headRev: number };
  const serverText = async (id: number) => (await server(id)).content.ops.map((op) => op.insert).join("");
  const providerCount = async () => (await (await request.get(`${provider}/control`)).json()).length as number;

  const universe = await post(`${api}/universes`, { title: `Script AI ${browserName}`, position: 0 });
  await post(`${api}/universes/${universe.id}/lore`, { kind: "place", name: "Kettle", visible_text: "The old copper kettle." });
  const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Kitchen script", position: 0, label: "screenplay", shape: "group" });
  const one = await post(`${api}/pieces/${group.id}/children`, { title: "INT. KITCHEN - DAY", position: 0, label: "scene" });
  const two = await post(`${api}/pieces/${group.id}/children`, { title: "EXT. GARDEN - NIGHT", position: 1, label: "scene" });
  await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Kettle boils.\n" }] } }, 200);
  await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Rain falls.\n" }] } }, 200);

  const editor = page.locator(".ql-editor");
  const stop = page.getByRole("button", { name: "Stop AI", exact: true });
  const instruction = page.getByRole("textbox", { name: "Continue instruction", exact: true });
  const inspector = page.getByRole("button", { name: "Context inspector", exact: true });
  const fillInstruction = async (text: string) => {
    if ((await inspector.getAttribute("aria-expanded")) !== "true") await inspector.click();
    await instruction.fill(text);
  };
  const expectScenes = async (oneText: string, twoText: string) => {
    await expect.poll(() => sceneTexts(page)).toEqual([oneText, twoText]);
    await expectSaved(page);
    await expect.poll(() => serverText(one.id)).toBe(oneText);
    await expect.poll(() => serverText(two.id)).toBe(twoText);
  };
  const sceneTwoStart = async () => 2 + (await sceneTexts(page))[0]!.length;
  const sceneOneEnd = async () => (await sceneTexts(page))[0]!.length;
  const sceneTwoEnd = async () => (await sceneTwoStart()) + (await sceneTexts(page))[1]!.length - 1;
  const arm = async (identity: string, model: string) => {
    const before = await providerCount();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model } })).status()).toBe(200);
    return before;
  };
  const upstream = async (id: number, identity: string, model: string) => {
    const recorded = await request.get(`${provider}/control/${id}/request`);
    expect(recorded.status()).toBe(200);
    expect(await recorded.json()).toEqual({ id, identity, model, stream: true, closed: false, completed: false });
    const control = `${provider}/control/${id}`;
    return {
      fragment: async (text: string) => expect((await request.post(`${control}/fragment`, { data: { text } })).status()).toBe(200),
      finish: async () => expect((await request.post(`${control}/finish`)).status()).toBe(200),
      closed: async () => (await (await request.get(`${control}/closed`)).json()) as { closed: boolean; completed: boolean },
    };
  };

  await page.addInitScript(() => {
    const NativeEventSource = window.EventSource;
    class ObservedEventSource extends NativeEventSource {
      constructor(url: string | URL, options?: EventSourceInit) {
        super(url, options);
        this.addEventListener("changed", event => {
          const state = window as unknown as { scriptChanges: number[] };
          state.scriptChanges.push(JSON.parse(event.data).pieceId);
        });
      }
    }
    Object.assign(window, { EventSource: ObservedEventSource, scriptChanges: [] });
  });
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
  await expectEditorReady(editor);
  await expect(editor.locator(".ql-scene-break")).toHaveCount(2);
  await attachQuill(page);
  await expectScenes("Kettle boils.\n", "Rain falls.\n");

  await test.step("the lore window reads scene 1 lore with the caret in scene 2", async () => {
    await select(page, (await sceneTwoStart()) + 4);
    await inspector.click();
    const list = page.getByRole("region", { name: "Lore in window", exact: true });
    await expect(list).toContainText("Kettle");
    await expect(list).toContainText("1 matches");
  });

  await test.step("Continue waits for a scene-two save and stays inside scene two", async () => {
    const identity = `GUZ985-Finish-${crypto.randomUUID()}`;
    await fillInstruction(identity);
    const before = await arm(identity, "e2e-continue");
    const postsBefore = posts.length;
    let held!: Route;
    const holding = new Promise<void>((resolve) => {
      void page.route(`${api}/pieces/${two.id}/deltas`, (route) => {
        held = route;
        resolve();
      });
    });
    await select(page, await sceneTwoStart());
    await page.keyboard.type("Mi");
    await holding;
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    await expect(stop).toBeVisible();
    await expect(page.getByText("Saving…", { exact: true })).toBeVisible();
    expect(await providerCount()).toBe(before);
    expect(await serverText(two.id)).toBe("Rain falls.\n");
    await held.continue();
    await page.unroute(`${api}/pieces/${two.id}/deltas`);
    await expect.poll(providerCount).toBe(before + 1);
    const run = await upstream(before, identity, "e2e-continue");
    await run.fragment("st. ");
    await run.finish();
    await expect(stop).toBeHidden();
    await expectScenes("Kettle boils.\n", "Mist. Rain falls.\n");
    expect(posts.slice(postsBefore).every((piece) => piece === two.id)).toBe(true);
    expect(posts.slice(postsBefore)).not.toHaveLength(0);
  });

  await test.step("Edit across a break is disabled; Edit inside scene 2 saves only scene 2", async () => {
    await select(page, (await sceneOneEnd()) - "boils.".length, "boils.\n\nMist.".length);
    await page.getByRole("button", { name: "Edit selection", exact: true }).click();
    const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
    await expect(note).toBeVisible();
    await expect(page.getByText(acrossBreak, { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Apply", exact: true })).toBeDisabled();
    await note.fill("Merge these");
    await note.press("Enter");
    await expect(note).toBeVisible();
    await note.press("Escape");
    await expect(note).toBeHidden();

    const identity = `GUZ985-edit-${crypto.randomUUID()}`;
    const rain = (await sceneTwoStart()) + "Mist. ".length;
    await select(page, rain, "Rain".length);
    await page.getByRole("button", { name: "Edit selection", exact: true }).click();
    await expect(page.getByText(acrossBreak, { exact: true })).toHaveCount(0);
    await note.fill(identity);
    const id = await arm(identity, "e2e-edit");
    const postsBefore = posts.length;
    await note.press("Enter");
    const run = await upstream(id, identity, "e2e-edit");
    await run.fragment("<replacement>Snow</replacement>");
    await run.finish();
    await expect(stop).toBeHidden();
    await expectScenes("Kettle boils.\n", "Mist. Snow falls.\n");
    expect(posts.slice(postsBefore).every((piece) => piece === two.id)).toBe(true);
    expect(posts.slice(postsBefore)).not.toHaveLength(0);
  });

  await test.step("Continue at the end of scene 1 pauses the poll and on Finish saves only scene 1", async () => {
    const identity = `GUZ985-Finish-${crypto.randomUUID()}`;
    await fillInstruction(identity);
    await select(page, await sceneOneEnd());
    const id = await arm(identity, "e2e-continue");
    const postsBefore = posts.length;
    await expectSaved(page);
    aiRunning = true;
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    const run = await upstream(id, identity, "e2e-continue");
    await run.fragment("Steam rises.");
    await expect(stop).toBeVisible();
    await expect.poll(() => sceneTexts(page)).toEqual(["Kettle boils.\nSteam rises.\n", "Mist. Snow falls.\n"]);
    await expect(editor).toHaveAttribute("data-ai-tint", "streaming");
    await expect(page.locator(".editor-continue-window")).toBeVisible();
    const remote = await server(two.id);
    const changes = await page.evaluate(id => (window as unknown as { scriptChanges: number[] }).scriptChanges.filter(piece => piece === id).length, two.id);
    await post(`${api}/pieces/${two.id}/deltas`, { baseRev: remote.headRev, delta: { ops: [{ insert: "Far " }] } }, 200);
    await expect.poll(() => page.evaluate(id => (window as unknown as { scriptChanges: number[] }).scriptChanges.filter(piece => piece === id).length, two.id)).toBeGreaterThan(changes);
    await run.fragment(" Lid rattles.");
    await expect.poll(() => sceneTexts(page)).toEqual(["Kettle boils.\nSteam rises. Lid rattles.\n", "Mist. Snow falls.\n"]);
    expect(await serverText(one.id)).toBe("Kettle boils.\n");
    expect(pollsWhileAi).toBe(0);
    aiRunning = false;
    await run.finish();
    await expect(stop).toBeHidden();
    expect(await run.closed()).toMatchObject({ closed: true, completed: true });
    await expectScenes("Kettle boils.\nSteam rises. Lid rattles.\n", "Far Mist. Snow falls.\n");
    expect(posts.slice(postsBefore)).not.toHaveLength(0);
    expect(posts.slice(postsBefore).every((piece) => piece === one.id)).toBe(true);
  });

  await test.step("Stop on Continue at the end of scene 2 leaves every scene unchanged", async () => {
    const identity = `GUZ985-Stop-${crypto.randomUUID()}`;
    await fillInstruction(identity);
    await select(page, await sceneTwoEnd());
    const id = await arm(identity, "e2e-continue");
    const postsBefore = posts.length;
    const heads = [(await server(one.id)).headRev, (await server(two.id)).headRev];
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    const run = await upstream(id, identity, "e2e-continue");
    await run.fragment("Thunder.");
    await expect.poll(() => sceneTexts(page)).toEqual(["Kettle boils.\nSteam rises. Lid rattles.\n", "Far Mist. Snow falls.\nThunder.\n"]);
    await stop.click();
    await expect(page.getByText("Cancelled", { exact: true })).toBeVisible();
    await expect.poll(() => run.closed()).toMatchObject({ closed: true, completed: false });
    await expectScenes("Kettle boils.\nSteam rises. Lid rattles.\n", "Far Mist. Snow falls.\n");
    expect([(await server(one.id)).headRev, (await server(two.id)).headRev]).toEqual(heads);
    expect(posts.slice(postsBefore)).toEqual([]);
  });

  expect(failures).toEqual([]);
});
