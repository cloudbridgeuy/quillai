import { readFileSync } from "node:fs";
import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

type SpecWindow = { specQuill: Quill };

const sceneOne = [
  ["INT. HALL - DAY", "scene-heading"],
  ["ANNA", "character"],
  ["quietly", "parenthetical"],
  ["Tom & Jerry wait.", "dialogue"],
  ["CUT TO:", "transition"],
] as const;
const sceneTwoText = "Rain falls.\n";

test("a two-scene script downloads as a Final Draft FDX in scene order", async ({ page, request, browserName }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const failures: string[] = [];
  page.on("pageerror", (error) => failures.push(error.message));
  page.on("response", (response) => {
    if (response.url().startsWith(api) && response.status() >= 400) failures.push(`${response.status()} ${response.url()}`);
  });
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  const universe = await post(`${api}/universes`, { title: `FDX ${browserName}`, position: 0 });
  const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Hall script", position: 0, label: "screenplay", shape: "group" });
  const one = await post(`${api}/pieces/${group.id}/children`, { title: "Hall", position: 0, label: "scene" });
  const two = await post(`${api}/pieces/${group.id}/children`, { title: "Yard", position: 1, label: "scene" });
  await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: sceneOne.map(([text]) => `${text}\n`).join("") }] } }, 200);
  await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: sceneTwoText }] } }, 200);

  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(group.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
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

  const blockType = page.getByLabel("Block type", { exact: true });
  let lineStart = 1;
  for (const [text, type] of sceneOne) {
    await page.evaluate((index) => {
      const quill = (window as unknown as SpecWindow).specQuill;
      quill.focus();
      quill.setSelection(index, 0, "user");
    }, lineStart);
    await blockType.selectOption(type);
    await expect(blockType).toHaveValue(type);
    lineStart += text.length + 1;
  }
  await expectSaved(page);

  const downloading = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download FDX", exact: true }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe("Hall script.fdx");
  const xml = readFileSync((await download.path())!, "utf8");
  if (browserName === "chromium") await download.saveAs("../target/guz-992/sample.fdx");
  expect(xml).toContain("Tom &amp; Jerry wait.");

  const parsed = await page.evaluate((source) => {
    const doc = new DOMParser().parseFromString(source, "application/xml");
    return {
      errors: doc.getElementsByTagName("parsererror").length,
      root: doc.documentElement.nodeName,
      paragraphs: Array.from(doc.querySelectorAll("FinalDraft > Content > Paragraph")).map((paragraph) => ({
        type: paragraph.getAttribute("Type"),
        text: paragraph.querySelector("Text")?.textContent,
      })),
    };
  }, xml);
  expect(parsed.errors).toBe(0);
  expect(parsed.root).toBe("FinalDraft");
  expect(parsed.paragraphs).toEqual([
    { type: "Scene Heading", text: "INT. HALL - DAY" },
    { type: "Character", text: "ANNA" },
    { type: "Parenthetical", text: "(quietly)" },
    { type: "Dialogue", text: "Tom & Jerry wait." },
    { type: "Transition", text: "CUT TO:" },
    { type: "Action", text: "Rain falls." },
  ]);
  expect(failures).toEqual([]);
});
