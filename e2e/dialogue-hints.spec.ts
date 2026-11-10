import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const api = process.env.QUILLAI_E2E_API_URL;

type Op = { insert?: unknown };
const opsText = (ops: Op[]) => ops.map((op) => (typeof op.insert === "string" ? op.insert : "")).join("");

async function send(request: APIRequestContext, method: "post" | "put", url: string, data: object) {
  const response = await request[method](url, { data });
  expect(response.ok(), await response.text()).toBe(true);
  return response.json();
}

async function openSeries(page: Page, request: APIRequestContext, width: number, rules: { language: "en" | "es"; rules: object }) {
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await send(request, "post", `${api}/series`, { title: "Dialogue hints", position: 0 });
  await send(request, "put", `${api}/series/${series.id}/rules`, rules);
  const book = await send(request, "post", `${api}/series/${series.id}/books`, { title: "Dialogue hints", position: 0 });
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await send(request, "post", chapters, { title: "Dialogue hints", position: 0, kind: "chapter", content: { ops: [{ insert: "\n" }] } });
  await page.setViewportSize({ width, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await editor.click();
  const savedText = async () => opsText((await (await request.get(`${chapters}/${chapter.id}`)).json()).content.ops);
  return { editor, savedText };
}

async function openTip(page: Page) {
  const underline = page.locator("[data-hint-underline]").first();
  await expect(underline).toBeVisible();
  const box = await underline.boundingBox();
  if (!box) throw new Error("underline has no box");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  const tip = page.getByRole("tooltip");
  await expect(tip).toBeVisible();
  return tip;
}

for (const width of [1100, 1440]) {
  test(`a Spanish dash tag with a period before it is underlined with its tip and the text is unchanged at ${width}px`, async ({ page, request }) => {
    const text = "—Hola. —dijo ella—.";
    const { savedText } = await openSeries(page, request, width, { language: "es", rules: {} });
    await page.keyboard.type(text);
    const tip = await openTip(page);
    await expect(tip).toContainText("Dash tag punctuation");
    await expect(tip).toContainText("No period or comma before a dialogue dash tag");
    await page.keyboard.press("Escape");
    await expect(tip).toBeHidden();
    await expect.poll(savedText).toBe(`${text}\n`);
  });

  test(`a said-bookism is underlined with its tip once the rule is on and the text is unchanged at ${width}px`, async ({ page, request }) => {
    const text = '"Stop," he hissed.';
    const { savedText } = await openSeries(page, request, width, { language: "en", rules: { "said-bookisms": { enabled: true } } });
    await page.keyboard.type(text);
    const tip = await openTip(page);
    await expect(tip).toContainText("Said-bookisms");
    await expect(tip).toContainText("hissed");
    await page.keyboard.press("Escape");
    await expect(tip).toBeHidden();
    await expect.poll(savedText).toBe(`${text.replace(/"(.*?),"/, "“$1,”")}\n`);
  });
}
