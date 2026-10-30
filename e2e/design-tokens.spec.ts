import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openEditor(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const seriesResponse = await request.post(`${api}/series`, { data: { title: "Design tokens", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const base = `${api}/series/${series.id}`;
  const bookResponse = await request.post(`${base}/books`, { data: { title: "Book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const content = `${Array.from({ length: 50 }, () => "Opening Mira arrived. ".repeat(8)).join("\n")}\n`;
  const chapterResponse = await request.post(`${base}/books/${book.id}/chapters`, { data: { title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: content }] } } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  expect((await request.post(`${base}/lore`, { data: { kind: "character", name: "Mira", visible_text: "The navigator.", level: "known_dont_reveal" } })).status()).toBe(201);
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.locator(".ql-editor p").first().click();
}

async function expectTopmost(locator: Locator, underneath?: Locator) {
  await expect(locator).toBeVisible();
  const box = await locator.boundingBox();
  const other = underneath ? await underneath.boundingBox() : box;
  if (!box || !other) throw new Error("Missing overlapping surface");
  const left = Math.max(box.x, other.x);
  const top = Math.max(box.y, other.y);
  const right = Math.min(box.x + box.width, other.x + other.width);
  const bottom = Math.min(box.y + box.height, other.y + other.height);
  expect(right).toBeGreaterThan(left);
  expect(bottom).toBeGreaterThan(top);
  const point = { x: (left + right) / 2, y: (top + bottom) / 2 };
  expect(await locator.evaluate((element, point) => {
    const hit = document.elementFromPoint(point.x, point.y);
    return hit === element || element.contains(hit);
  }, point)).toBe(true);
  await locator.page().mouse.click(point.x, point.y);
  await expect(locator).toBeVisible();
}

for (const width of [1100, 1440]) {
  test(`visible surfaces use design tokens at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await expect(page.locator(".editor-rail-left")).toHaveCSS("padding-left", "12px");
    await expect(page.locator(".editor-topbar")).toHaveCSS("margin-top", "8px");
    await expect(page.getByRole("button", { name: "Continue writing", exact: true })).toHaveCSS("padding-top", "4px");
    const bracket = page.locator(".editor-continue-window");
    await expect(bracket).toBeVisible();
    await expect(bracket).toBeInViewport();
    await expect(bracket).toHaveCSS("border-top-left-radius", "4px");
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(page.locator("#manuscript-drawer")).toHaveCSS("padding-top", "16px");
    await expect(page.getByRole("region", { name: "Manuscript outline" }).getByRole("button", { name: "Chapter", exact: true })).toHaveCSS("padding-top", "8px");
    await expect(page.getByRole("region", { name: "Series lore" }).getByRole("button", { name: /Mira/ })).toHaveCSS("padding-top", "8px");
    await expect(page.getByRole("region", { name: "Series lore" }).getByRole("button", { name: "New lore" })).toBeVisible();
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    await expect(page.locator(".editor-context-inspector > .editor-drawer-header")).toHaveCSS("padding-left", "16px");
    await page.getByRole("region", { name: "Lore in window" }).getByRole("button", { name: /Mira/ }).click();
    const lore = page.getByRole("dialog", { name: "Lore match" });
    await expect(lore).toHaveCSS("padding-left", "16px");
    await expect(lore).toHaveCSS("border-radius", "4px");
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings", exact: true });
    await expect(settings).toHaveCSS("padding-top", "16px");
    await expect(settings).toHaveCSS("bottom", "20px");
    await settings.getByRole("checkbox", { name: "Whole document" }).check();
    await page.keyboard.press("Escape");
    const editor = page.locator(".ql-editor");
    await editor.evaluate(element => {
      const text = element.querySelector("p")!.firstChild!;
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 7);
      window.getSelection()!.removeAllRanges();
      window.getSelection()!.addRange(range);
    });
    await page.getByRole("button", { name: "Edit selection" }).click();
    const edit = page.getByRole("dialog", { name: "Edit instruction" });
    await expect(edit).toHaveCSS("padding-left", "16px");
    await expect(edit).toHaveCSS("border-radius", "4px");
    await page.keyboard.press("Escape");
    await editor.evaluate(element => {
      const node = element.querySelectorAll("p")[30]!.firstChild!;
      (element as HTMLElement).focus({ preventScroll: true });
      const range = document.createRange();
      range.setStart(node, 10);
      range.collapse(true);
      window.getSelection()!.removeAllRanges();
      window.getSelection()!.addRange(range);
    });
    await page.locator(".ql-editor p").nth(30).scrollIntoViewIfNeeded();
    const flag = page.getByRole("button", { name: /Continue window starts ↑/ });
    await expect(flag).toHaveCSS("padding-top", "4px");
    await expect(flag).toHaveCSS("padding-left", "12px");
  });

  test(`popovers remain above the inspector at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    const drawer = page.locator("#context-drawer");
    await expect.poll(() => page.evaluate(() => CSS.highlights.get("quillai-lore-known_dont_reveal")?.size ?? 0)).toBeGreaterThan(0);
    const point = await page.evaluate(() => {
      const rects = [...CSS.highlights.get("quillai-lore-known_dont_reveal")!]
        .flatMap(range => [...range.getClientRects()])
        .filter(rect => rect.top >= 0 && rect.bottom <= window.innerHeight)
        .sort((left, right) => right.right - left.right);
      const rect = rects[0];
      if (!rect) throw new Error("Missing visible lore highlight");
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    });
    await page.mouse.move(point.x, point.y);
    await expectTopmost(page.getByRole("dialog", { name: "Lore match" }), width === 1100 ? drawer : undefined);
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const settings = page.getByRole("dialog", { name: "Settings", exact: true });
    await expect(drawer).toBeHidden();
    await expectTopmost(settings);
    await settings.getByRole("checkbox", { name: "Whole document" }).check();
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(settings).toBeHidden();
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    await page.locator(".ql-editor").evaluate(element => {
      const text = element.querySelector("p")!.firstChild!;
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 7);
      window.getSelection()!.removeAllRanges();
      window.getSelection()!.addRange(range);
    });
    await page.getByRole("button", { name: "Edit selection" }).click();
    await expectTopmost(page.getByRole("dialog", { name: "Edit instruction" }), drawer);
  });

  test(`modal remains above the rail and drawer at ${width}px`, async ({ page, request }) => {
    await openEditor(page, request, width);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await page.getByRole("region", { name: "Series lore" }).getByRole("button", { name: "New lore" }).click();
    const modal = page.getByRole("dialog", { name: "Create lore" });
    await expectTopmost(modal.getByRole("heading", { name: "Create lore", exact: true }));
    const rail = page.getByRole("button", { name: "Manuscript", exact: true });
    for (const underneath of [rail, page.locator("#manuscript-drawer")]) {
      expect(await underneath.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
        const dialog = document.querySelector("dialog[open]");
        return hit === dialog || !!dialog?.contains(hit);
      })).toBe(true);
    }
    const box = await rail.boundingBox();
    if (!box) throw new Error("Missing rail");
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await expect(modal).toHaveCount(0);
    await expect(rail).toHaveAttribute("aria-expanded", "true");
  });
}
