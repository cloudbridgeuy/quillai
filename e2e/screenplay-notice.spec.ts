import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

type Op = { insert?: unknown; attributes?: Record<string, unknown> };
type SpecWindow = { specQuill: Quill; specMarker?: string };

const NOTICE = "Edits across a scene break are not saved";
const sceneOne = "INT. HALL - DAY";
const sceneTwo = "EXT. YARD - NIGHT";
const longScene = Array.from({ length: 40 }, (_, line) => `Line ${line + 1} of the hall.\n`).join("");

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
  test(`the scene notice sits clear of prose and blocked edits keep undo at ${width}px`, async ({ page, request, browserName }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const failures: string[] = [];
    let navigations = 0;
    page.on("pageerror", (error) => failures.push(error.message));
    page.on("response", (response) => {
      if (response.url().startsWith(api) && response.status() >= 400) failures.push(`${response.status()} ${response.url()}`);
    });
    page.on("framenavigated", (frame) => { if (frame === page.mainFrame()) navigations++; });
    const post = async (url: string, data: object, status = 201) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(status);
      return response.json();
    };
    const serverText = async (id: number) => (await (await request.get(`${api}/pieces/${id}`)).json()).content.ops.map((op: Op) => op.insert).join("");

    const universe = await post(`${api}/universes`, { title: `Notice ${browserName} ${width}`, position: 0 });
    const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Yard script", position: 0, label: "screenplay", shape: "group" });
    const one = await post(`${api}/pieces/${group.id}/children`, { title: sceneOne, position: 0, label: "scene" });
    const two = await post(`${api}/pieces/${group.id}/children`, { title: sceneTwo, position: 1, label: "scene" });
    await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: longScene }] } }, 200);
    await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Rain falls.\n" }] } }, 200);

    const editor = page.locator(".ql-editor");
    const breaks = editor.locator(".ql-scene-break");
    const notice = page.getByRole("alert").filter({ hasText: NOTICE });
    const drawer = page.locator("#manuscript-drawer");
    const manuscript = page.getByRole("button", { name: "Manuscript", exact: true });
    const twoStart = 2 + longScene.length;
    const caret = async () => (await contents(page)).selection;
    const expectBoth = async (oneText: string, twoText: string) => {
      await expect.poll(async () => sceneSlices((await contents(page)).ops)).toEqual({ breaks: [one.id, two.id], scenes: [oneText, twoText] });
      await expect.poll(() => serverText(one.id)).toBe(oneText);
      await expect.poll(() => serverText(two.id)).toBe(twoText);
    };
    const across = () => select(page, longScene.length - "hall.".length, "hall.\n\nRain".length);
    const typeInSceneTwo = async (text: string) => {
      await select(page, twoStart);
      await page.keyboard.type(text);
      await expect(notice).toBeHidden();
      await expectBoth(longScene, `${text}Rain falls.\n`);
    };
    const undoRestoresSceneTwo = async () => {
      await select(page, twoStart);
      await page.keyboard.press("ControlOrMeta+z");
      await expectBoth(longScene, "Rain falls.\n");
    };
    const shownOnce = () => page.evaluate((text) => document.body.innerText.split(text).length - 1, NOTICE);

    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
    await expectEditorReady(editor);
    await expect(breaks).toHaveCount(2);
    await attachQuill(page);
    await page.evaluate(() => { (window as unknown as SpecWindow).specMarker = "kept"; });
    const loaded = navigations;

    await test.step("a tree click on a scene of the open script moves the caret in place and highlights that scene", async () => {
      await manuscript.click();
      const sceneButton = (name: string) => drawer.getByRole("button", { name, exact: true });
      await expect(sceneButton(sceneOne)).toHaveAttribute("aria-pressed", "true");
      await sceneButton(sceneTwo).click();
      await expect.poll(caret).toEqual({ index: twoStart, length: 0 });
      await expect(sceneButton(sceneTwo)).toHaveAttribute("aria-pressed", "true");
      await expect(sceneButton(sceneOne)).toHaveAttribute("aria-pressed", "false");
      await expect(sceneButton("Yard script")).toHaveAttribute("aria-pressed", "false");
      await sceneButton(sceneOne).click();
      await expect.poll(caret).toEqual({ index: 1, length: 0 });
      await expect(sceneButton(sceneOne)).toHaveAttribute("aria-pressed", "true");
      await expect(sceneButton(sceneTwo)).toHaveAttribute("aria-pressed", "false");
      expect(navigations).toBe(loaded);
      expect(await page.evaluate(() => (window as unknown as SpecWindow).specMarker)).toBe("kept");
      await manuscript.click();
      await expect(drawer).toBeHidden();
    });

    await test.step("a context-menu cut across the break is blocked before it applies and undo still works", async () => {
      await typeInSceneTwo("Wind. ");
      await across();
      await page.evaluate(() => {
        const event = new ClipboardEvent("cut", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
        (window as unknown as SpecWindow).specQuill.root.dispatchEvent(event);
      });
      await expect(notice).toBeVisible();
      await expect.poll(shownOnce).toBe(1);
      await expectBoth(longScene, "Wind. Rain falls.\n");
      await undoRestoresSceneTwo();
    });

    await test.step("a drag across the break and a drop on the break line are blocked and undo still works", async () => {
      await typeInSceneTwo("Dusk. ");
      await across();
      const selected = await editor.evaluate(() => window.getSelection()!.getRangeAt(0).getClientRects()[0]!.toJSON() as DOMRect);
      const target = await editor.locator("p").last().boundingBox();
      await page.mouse.move(selected.x + selected.width / 2, selected.y + selected.height / 2);
      await page.mouse.down();
      await page.mouse.move(target!.x + target!.width / 2, target!.y + target!.height / 2, { steps: 8 });
      await page.mouse.up();
      await expectBoth(longScene, "Dusk. Rain falls.\n");
      const prevented = await breaks.nth(1).evaluate((node) => {
        const data = new DataTransfer();
        data.setData("text/plain", "Dropped");
        const event = new DragEvent("drop", { dataTransfer: data, bubbles: true, cancelable: true });
        node.dispatchEvent(event);
        return event.defaultPrevented;
      });
      expect(prevented).toBe(true);
      await expect(notice).toBeVisible();
      await expect.poll(shownOnce).toBe(1);
      await expectBoth(longScene, "Dusk. Rain falls.\n");
      await undoRestoresSceneTwo();
    });

    await test.step("the notice never covers the last line of a long scene and the page does not shift", async () => {
      await select(page, twoStart);
      await page.keyboard.type("A");
      await expect(notice).toBeHidden();
      await page.keyboard.press("Backspace");
      await expectBoth(longScene, "Rain falls.\n");
      const sheet = page.locator(".editor-page");
      const before = await sheet.evaluate((node) => node.getBoundingClientRect().top + window.scrollY);
      await select(page, longScene.length);
      await page.keyboard.press("Delete");
      await expect(notice).toBeVisible();
      await expect.poll(shownOnce).toBe(1);
      expect(await sheet.evaluate((node) => node.getBoundingClientRect().top + window.scrollY)).toBe(before);
      const lastLine = breaks.nth(1).locator("xpath=preceding-sibling::*[1]");
      await expect(lastLine).toHaveText("Line 40 of the hall.");
      await lastLine.scrollIntoViewIfNeeded();
      const pill = (await notice.boundingBox())!;
      const line = (await lastLine.boundingBox())!;
      await page.evaluate((delta) => window.scrollBy(0, delta), (line.y + line.height / 2) - (pill.y + pill.height / 2));
      const pillAfter = (await notice.boundingBox())!;
      const lineAfter = (await lastLine.boundingBox())!;
      const overlaps = pillAfter.x < lineAfter.x + lineAfter.width && lineAfter.x < pillAfter.x + pillAfter.width
        && pillAfter.y < lineAfter.y + lineAfter.height && lineAfter.y < pillAfter.y + pillAfter.height;
      expect(overlaps, JSON.stringify({ pill: pillAfter, line: lineAfter })).toBe(false);
      const title = (await page.locator(".editor-chapter-title").boundingBox())!;
      expect(pillAfter.y + pillAfter.height).toBeLessThanOrEqual(title.y);
      await page.evaluate(() => window.scrollTo(0, 0));
      await expect(notice).toBeInViewport();
      await page.screenshot({ path: `../target/guz-988/${test.info().project.name}/scene-notice-${width}.png` });
      await expectBoth(longScene, "Rain falls.\n");
    });

    expect(failures).toEqual([]);
  });
}
