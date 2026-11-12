import { readFileSync } from "node:fs";
import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type SpecWindow = { specQuill: Quill };

const headingText = "INT. HALL - DAY\n";
const sceneTwoText = "Rain falls.\n";

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
  return page.evaluate(() => (window as unknown as SpecWindow).specQuill.getContents().ops as Op[]);
}

async function select(page: Page, index: number, length = 0) {
  await page.evaluate(([index, length]) => {
    const quill = (window as unknown as SpecWindow).specQuill;
    quill.focus();
    quill.setSelection(index!, length!, "user");
  }, [index, length]);
}

function blocks(ops: Op[]): unknown[] {
  return ops.flatMap((op) => (op.attributes?.["script-block"] === undefined ? [] : [op.attributes["script-block"]]));
}

function lineEnd(ops: Op[], block: string): number {
  let index = 0;
  for (const op of ops) {
    if (op.attributes?.["script-block"] === block) return index;
    index += typeof op.insert === "string" ? op.insert.length : 1;
  }
  throw new Error(`No ${block} line`);
}

for (const width of [1100, 1440]) {
  test.describe(`script block formats at ${width}px`, () => {
    const api = process.env.QUILLAI_E2E_API_URL;

    async function seed(request: APIRequestContext, label: string) {
      if (!api) throw new Error("Run with cargo xtask e2e");
      const post = async (url: string, data: object, status = 201) => {
        const response = await request.post(url, { data });
        expect(response.status(), await response.text()).toBe(status);
        return response.json();
      };
      const universe = await post(`${api}/universes`, { title: `Format ${label}`, position: 0 });
      const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Hall script", position: 0, label: "screenplay", shape: "group" });
      const one = await post(`${api}/pieces/${group.id}/children`, { title: "Hall", position: 0, label: "scene" });
      const two = await post(`${api}/pieces/${group.id}/children`, { title: "Yard", position: 1, label: "scene" });
      await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: headingText }] } }, 200);
      await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: sceneTwoText }] } }, 200);
      const serverOps = async (id: number): Promise<Op[]> => (await (await request.get(`${api}/pieces/${id}`)).json()).content.ops;
      return { group, one, two, serverOps };
    }

    async function open(page: Page, groupId: number) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api!, pieceId: String(groupId) })}`);
      await expectEditorReady(page.locator(".ql-editor"));
      await attachQuill(page);
    }

    test("block types from the select, Tab and Enter save, reload and export as Fountain", async ({ page, request, browserName }) => {
      const failures: string[] = [];
      page.on("pageerror", (error) => failures.push(error.message));
      page.on("response", (response) => {
        if (response.url().startsWith(api!) && response.status() >= 400) failures.push(`${response.status()} ${response.url()}`);
      });
      const { group, one, two, serverOps } = await seed(request, `${browserName} ${width} types`);
      await open(page, group.id);
      const blockType = page.getByLabel("Block type", { exact: true });
      await select(page, 1);
      await blockType.selectOption("scene-heading");
      await select(page, headingText.length);
      await page.keyboard.press("Enter");
      await expect(blockType).toHaveValue("action");
      await page.keyboard.press("Tab");
      await expect(blockType).toHaveValue("character");
      await page.keyboard.type("ANNA");
      await page.keyboard.press("Enter");
      await expect(blockType).toHaveValue("dialogue");
      await page.keyboard.type("Good morning.");

      await expect.poll(async () => blocks(await serverOps(one.id))).toEqual(["scene-heading", "character", "dialogue"]);
      await expect.poll(async () => (await serverOps(one.id)).map((op) => op.insert).join("")).toBe("INT. HALL - DAY\nANNA\nGood morning.\n");
      expect(await serverOps(two.id)).toEqual([{ insert: sceneTwoText }]);
      await page.screenshot({ path: `../target/guz-944/${test.info().project.name}/format-${width}.png` });

      await open(page, group.id);
      expect(blocks(await contents(page))).toEqual(["scene-heading", "character", "dialogue"]);
      await select(page, lineEnd(await contents(page), "dialogue"));
      await expect(blockType).toHaveValue("dialogue");
      await expect(page.locator(".ql-editor .ql-script-block-dialogue")).toHaveText("Good morning.");

      const downloading = page.waitForEvent("download");
      await page.getByRole("link", { name: "Download Fountain", exact: true }).click();
      const download = await downloading;
      expect(download.suggestedFilename()).toBe("Hall script.fountain");
      expect(readFileSync((await download.path())!, "utf8")).toBe("INT. HALL - DAY\n\nANNA\nGood morning.\n\nRain falls.\n");
      expect(failures).toEqual([]);
    });

    if (width === 1100) for (const scenario of ["remote", "continue", "edit", "edit newline"] as const) {
      test(scenario === "edit newline" ? "Edit including a heading newline keeps the heading line ending and survives reload" : `block types survive ${scenario} and reload`, async ({ page, request, browserName }) => {
        const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
        if (!provider) throw new Error("Run with cargo xtask e2e and its default catalog");
        const { group, one, two } = await seed(request, `${browserName} ${scenario}`);
        const getScenes = async () => (await (await request.get(`${api}/pieces/${group.id}/scenes`)).json()).scenes as Array<{ id: number; headRev: number; content: { ops: Op[] } }>;
        const lineTypes = (ops: Op[]) => {
          const lines: Array<{ text: string; type: unknown }> = [];
          let text = "";
          for (const op of ops) {
            if (op.attributes?.["scene-break"] !== undefined) continue;
            for (const character of typeof op.insert === "string" ? op.insert : "") {
              if (character === "\n") {
                lines.push({ text, type: op.attributes?.["script-block"] });
                text = "";
              } else text += character;
            }
          }
          return lines;
        };
        await open(page, group.id);
        await select(page, 1);
        await page.keyboard.type("Typed ");
        await page.getByLabel("Block type", { exact: true }).selectOption("scene-heading");
        await page.evaluate((index) => {
          const quill = (window as unknown as SpecWindow).specQuill;
          quill.insertText(index, "\nANNA\nHello.\n(quietly)\nCUT TO:\nAction.", "user");
          const lines = [["Typed INT. HALL - DAY", "scene-heading"], ["ANNA", "character"], ["Hello.", "dialogue"], ["(quietly)", "parenthetical"], ["CUT TO:", "transition"], ["Action.", false]] as const;
          let offset = 1;
          for (const [text, type] of lines) {
            quill.formatLine(offset, 1, "script-block", type, "user");
            offset += text.length + 1;
          }
        }, headingText.length + "Typed ".length);
        const initial = [
          { text: "Typed INT. HALL - DAY", type: "scene-heading" },
          { text: "ANNA", type: "character" },
          { text: "Hello.", type: "dialogue" },
          { text: "(quietly)", type: "parenthetical" },
          { text: "CUT TO:", type: "transition" },
          { text: "Action.", type: undefined },
          { text: "Rain falls.", type: undefined },
        ];
        await expect.poll(async () => lineTypes(await contents(page))).toEqual(initial);
        await expect.poll(async () => lineTypes((await getScenes()).flatMap((scene) => scene.content.ops))).toEqual(initial);
        await expectSaved(page);
        let expected = initial;
        if (scenario === "remote") {
          const scene = (await getScenes()).find((item) => item.id === one.id)!;
          const response = await request.post(`${api}/pieces/${one.id}/deltas`, { data: { baseRev: scene.headRev, delta: { ops: [{ insert: "Before\n" }] } } });
          expect(response.status(), await response.text()).toBe(200);
          expected = [{ text: "Before", type: undefined }, ...initial];
        } else {
          const edit = scenario.startsWith("edit");
          const identity = `GUZ985-${edit ? "edit" : "Finish"}-${crypto.randomUUID()}`;
          const model = edit ? "e2e-edit" : "e2e-continue";
          const id = (await (await request.get(`${provider}/control`)).json()).length as number;
          if (edit) {
            await select(page, scenario === "edit newline" ? 1 : 1 + "Typed ".length, scenario === "edit newline" ? "Typed INT. HALL - DAY\n".length : "INT. HALL".length);
            await page.getByRole("button", { name: "Edit selection", exact: true }).click();
            await page.getByRole("textbox", { name: "Edit instruction", exact: true }).fill(identity);
          } else {
            await page.getByRole("button", { name: "Context inspector", exact: true }).click();
            await page.getByRole("textbox", { name: "Continue instruction", exact: true }).fill(identity);
            await select(page, (await contents(page)).reduce((length, op) => length + (typeof op.insert === "string" ? op.insert.length : 1), 0) - sceneTwoText.length - 2);
          }
          expect((await request.post(`${provider}/control/arm`, { data: { identity, model } })).status()).toBe(200);
          if (edit) await page.getByRole("textbox", { name: "Edit instruction", exact: true }).press("Enter");
          else await page.getByRole("button", { name: "Continue writing", exact: true }).click();
          const upstream = await request.get(`${provider}/control/${id}/request`);
          expect(await upstream.json()).toMatchObject({ id, identity, model });
          expect((await request.post(`${provider}/control/${id}/fragment`, { data: { text: edit ? "<replacement>Revised</replacement>" : "More." } })).status()).toBe(200);
          expect((await request.post(`${provider}/control/${id}/finish`)).status()).toBe(200);
          await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
          expected = edit
            ? [{ text: scenario === "edit newline" ? "Revised" : "Typed Revised - DAY", type: "scene-heading" }, ...initial.slice(1)]
            : [...initial.slice(0, -1), { text: "More.", type: undefined }, initial[initial.length - 1]!];
        }
        await expect.poll(async () => lineTypes(await contents(page))).toEqual(expected);
        await expect.poll(async () => lineTypes((await getScenes()).flatMap((scene) => scene.content.ops))).toEqual(expected);
        await open(page, group.id);
        expect(lineTypes(await contents(page))).toEqual(expected);
        const scenes = await getScenes();
        expect(scenes.map((scene) => scene.id)).toEqual([one.id, two.id]);
        expect(lineTypes(scenes.flatMap((scene) => scene.content.ops))).toEqual(expected);
      });
    }

    test("markdown shortcuts stay off on the script surface", async ({ page, request, browserName }) => {
      const { group, two, serverOps } = await seed(request, `${browserName} ${width} markdown`);
      await open(page, group.id);
      await select(page, 2 + headingText.length);
      await page.keyboard.type("# hi");
      await page.keyboard.press("Enter");
      await page.keyboard.type("- item");
      await page.keyboard.press("Enter");
      await page.keyboard.type("**x** y");
      await page.keyboard.press("Enter");
      await expect.poll(async () => (await serverOps(two.id)).map((op) => op.insert).join("")).toBe(`# hi\n- item\n**x** y\n${sceneTwoText}`);
      const saved = JSON.stringify(await serverOps(two.id));
      for (const name of ["header", "list", "bold", "script-block"]) expect(saved).not.toContain(`"${name}"`);
      await expect(page.locator(".ql-editor h1, .ql-editor li, .ql-editor strong")).toHaveCount(0);
      await page.screenshot({ path: `../target/guz-944/${test.info().project.name}/format-markdown-${width}.png` });
    });

    test("a block format on the break line is refused and never saved", async ({ page, request, browserName }) => {
      const { group, one, two, serverOps } = await seed(request, `${browserName} ${width} break`);
      await open(page, group.id);
      await page.evaluate(() => (window as unknown as SpecWindow).specQuill.formatLine(0, 1, "script-block", "dialogue", "user"));
      await expect(page.getByRole("status").filter({ hasText: "Edits across a scene break are not saved" })).toBeVisible();
      await expect.poll(async () => blocks(await contents(page))).toEqual([]);
      await page.waitForTimeout(1500);
      expect(blocks(await serverOps(one.id))).toEqual([]);
      expect(blocks(await serverOps(two.id))).toEqual([]);
      await open(page, group.id);
      expect(blocks(await contents(page))).toEqual([]);
      await page.screenshot({ path: `../target/guz-944/${test.info().project.name}/format-break-${width}.png` });
    });
  });
}
