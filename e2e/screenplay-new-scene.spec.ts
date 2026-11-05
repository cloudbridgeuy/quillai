import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type SpecWindow = { specQuill: Quill };

const sceneOne = "INT. HALL - DAY";
const sceneTwo = "EXT. YARD - NIGHT";
const textOne = "Kettle boils.\n";
const textTwo = "Rain falls.\n";
const sceneThree = "INT. CELLAR - NIGHT";
const textThree = "Drip drip.\n";

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

for (const width of [1100, 1440]) {
  test.describe(`new scene at ${width}px`, () => {
    const api = process.env.QUILLAI_E2E_API_URL;

    async function seed(request: import("../app/node_modules/@playwright/test/index.js").APIRequestContext, label: string, withThree = false) {
      if (!api) throw new Error("Run with cargo xtask e2e");
      const post = async (url: string, data: object, status = 201) => {
        const response = await request.post(url, { data });
        expect(response.status(), await response.text()).toBe(status);
        return response.json();
      };
      const universe = await post(`${api}/universes`, { title: `New scene ${label}`, position: 0 });
      const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Yard script", position: 0, label: "screenplay", shape: "group" });
      const one = await post(`${api}/pieces/${group.id}/children`, { title: sceneOne, position: 0, label: "scene" });
      const two = await post(`${api}/pieces/${group.id}/children`, { title: sceneTwo, position: 1, label: "scene" });
      await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: textOne }] } }, 200);
      await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: textTwo }] } }, 200);
      const three = withThree ? await post(`${api}/pieces/${group.id}/children`, { title: sceneThree, position: 2, label: "scene" }) : null;
      if (three) await post(`${api}/pieces/${three.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: textThree }] } }, 200);
      const scenes = async (): Promise<{ id: number; position: number; content: { ops: Op[] } }[]> =>
        (await (await request.get(`${api}/pieces/${group.id}/scenes`)).json()).scenes;
      const order = async () => (await scenes()).map((scene) => scene.id);
      const text = async (id: number) => (await scenes()).find((scene) => scene.id === id)!.content.ops.map((op) => op.insert).join("");
      return { group, one, two, three, scenes, order, text };
    }

    async function open(page: Page, groupId: number) {
      await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api!, pieceId: String(groupId) })}`);
      await expectEditorReady(page.locator(".ql-editor"));
      await attachQuill(page);
    }

    test("adds a scene after the caret scene and moves the caret into it", async ({ page, request, browserName }) => {
      const failures: string[] = [];
      page.on("pageerror", (error) => failures.push(error.message));
      page.on("response", (response) => {
        if (response.url().startsWith(api!) && response.status() >= 400) failures.push(`${response.status()} ${response.url()}`);
      });
      const { group, one, two, order, text } = await seed(request, `${browserName} ${width} add`);
      await page.setViewportSize({ width, height: 900 });
      await open(page, group.id);
      const editor = page.locator(".ql-editor");
      await select(page, 1);
      await page.getByRole("button", { name: "New scene", exact: true }).click();
      await expect.poll(async () => (await order())[2]).toBe(two.id);
      const added = (await order())[1]!;
      expect((await order())[0]).toBe(one.id);
      await expect(editor.locator(".ql-scene-break")).toHaveCount(3);
      await attachQuill(page);
      await expect.poll(async () => sceneSlices((await contents(page)).ops)).toEqual({ breaks: [one.id, added, two.id], scenes: [textOne, "\n", textTwo] });
      await expect.poll(async () => (await contents(page)).selection).toEqual({ index: 2 + textOne.length, length: 0 });

      await page.keyboard.type("Wind.");
      await expect.poll(() => text(added)).toBe("Wind.\n");
      expect(await text(one.id)).toBe(textOne);
      expect(await text(two.id)).toBe(textTwo);

      await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      const drawer = page.locator("#manuscript-drawer");
      const tops: number[] = [];
      for (const name of [sceneOne, "Scene 3", sceneTwo]) tops.push((await drawer.getByRole("button", { name, exact: true }).boundingBox())!.y);
      expect(tops).toEqual([...tops].sort((a, b) => a - b));
      await page.screenshot({ path: `../target/guz-986/${test.info().project.name}/new-scene-${width}.png` });

      await open(page, group.id);
      await expect(editor.locator(".ql-scene-break")).toHaveCount(3);
      await expect.poll(async () => sceneSlices((await contents(page)).ops).breaks).toEqual([one.id, added, two.id]);
      expect(failures).toEqual([]);
    });

    async function expectOrderKept(page: Page, group: { id: number }, expected: { id: number; title: string; text: string }[], order: () => Promise<number[]>, text: (id: number) => Promise<string>) {
      await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api!, pieceId: String(group.id) })}`);
      await expectEditorReady(page.locator(".ql-editor"));
      await attachQuill(page);
      expect(await order()).toEqual(expected.map(({ id }) => id));
      for (const { id, text: want } of expected) expect(await text(id)).toBe(want);
      await expect.poll(async () => sceneSlices((await contents(page)).ops)).toEqual({
        breaks: expected.map(({ id }) => id), scenes: expected.map(({ text: want }) => want),
      });
      await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      const drawer = page.locator("#manuscript-drawer");
      const tops: number[] = [];
      for (const { title } of expected) tops.push((await drawer.getByRole("button", { name: title, exact: true }).boundingBox())!.y);
      expect(tops).toEqual([...tops].sort((a, b) => a - b));
      expect(new Set(tops).size).toBe(expected.length);
    }

    test("a failed create shows an error and keeps the exact order", async ({ page, request, browserName }) => {
      const { group, one, two, order, text } = await seed(request, `${browserName} ${width} create`);
      await page.setViewportSize({ width, height: 900 });
      await page.route(`${api}/pieces/${group.id}/children`, (route) => route.fulfill({
        status: 500, contentType: "application/json", body: JSON.stringify({ error: "boom", code: "INTERNAL" }),
      }));
      await open(page, group.id);
      await select(page, 1);
      await page.getByRole("button", { name: "New scene", exact: true }).click();
      await expect(page.getByRole("alert").filter({ hasText: "New scene not added" })).toBeVisible();
      await page.screenshot({ path: `../target/guz-986/${test.info().project.name}/new-scene-create-failed-${width}.png` });
      expect(sceneSlices((await contents(page)).ops)).toEqual({ breaks: [one.id, two.id], scenes: [textOne, textTwo] });
      await page.unroute(`${api}/pieces/${group.id}/children`);
      await expectOrderKept(page, group, [
        { id: one.id, title: sceneOne, text: textOne }, { id: two.id, title: sceneTwo, text: textTwo },
      ], order, text);
    });

    for (const failing of [0, 1]) {
      test(`a failed renumber PUT number ${failing + 1} adds no scene and keeps the exact order`, async ({ page, request, browserName }) => {
        const { group, one, two, three, order, text } = await seed(request, `${browserName} ${width} renumber ${failing}`, true);
        await page.setViewportSize({ width, height: 900 });
        let puts = 0;
        await page.route(new RegExp(`^${api}/pieces/\\d+$`), (route) => {
          if (route.request().method() !== "PUT") return route.continue();
          return puts++ === failing
            ? route.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify({ error: "boom", code: "INTERNAL" }) })
            : route.continue();
        });
        await open(page, group.id);
        await select(page, 1);
        await page.getByRole("button", { name: "New scene", exact: true }).click();
        await expect(page.getByRole("alert").filter({ hasText: "New scene not added" })).toBeVisible();
        expect(puts).toBe(failing + 1);
        await page.unroute(new RegExp(`^${api}/pieces/\\d+$`));
        await expectOrderKept(page, group, [
          { id: one.id, title: sceneOne, text: textOne }, { id: two.id, title: sceneTwo, text: textTwo },
          { id: three!.id, title: sceneThree, text: textThree },
        ], order, text);
      });
    }

    test("another tab adding a scene shows Scenes changed without rebuilding the surface", async ({ page, request, browserName }) => {
      const { group, one, two } = await seed(request, `${browserName} ${width} changed`);
      await page.setViewportSize({ width, height: 900 });
      await open(page, group.id);
      const response = await request.post(`${api}/pieces/${group.id}/children`, { data: { title: "Elsewhere", position: 2, label: "scene" } });
      expect(response.status()).toBe(201);
      const changed = page.getByRole("status").filter({ hasText: "Scenes changed; reload" });
      // Adding a scene emits no SSE nudge; the 15s fallback poll (GUZ-1194) detects it.
      await expect(changed).toBeVisible({ timeout: 25_000 });
      await page.keyboard.type("x");
      await expect(changed).toBeVisible();
      expect(sceneSlices((await contents(page)).ops).breaks).toEqual([one.id, two.id]);
      await page.screenshot({ path: `../target/guz-986/${test.info().project.name}/scenes-changed-${width}.png` });
    });
  });
}
