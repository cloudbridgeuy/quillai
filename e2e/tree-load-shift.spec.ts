import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`manuscript loading reserves capped space without moving lore or export at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const seriesResponse = await request.post(`${api}/series`, { data: { title: `Tree ${width}`, position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const base = `${api}/series/${series.id}`;
    const bookResponse = await request.post(`${base}/books`, { data: { title: "Large outline", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chaptersUrl = `${base}/books/${book.id}/chapters`;
    const chapterResponse = await request.post(chaptersUrl, { data: {
      title: "Current chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Local chapter text.\n" }] },
    } });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    expect((await request.post(`${base}/lore`, { data: {
      kind: "character", name: "Mira", visible_text: "The navigator.",
    } })).status()).toBe(201);
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    let release!: () => void;
    let intercepted!: () => void;
    const held = new Promise<void>(resolve => { release = resolve; });
    const treeRequested = new Promise<void>(resolve => { intercepted = resolve; });
    await page.route(`${api}/series`, async route => {
      intercepted();
      await held;
      await route.fulfill({ json: [series] });
    });
    await page.route(chaptersUrl, route => route.fulfill({ json: [chapter, ...Array.from({ length: 150 }, (_, index) => ({
      id: chapter.id + index + 1, title: `Outline chapter ${index + 1}`, position: index + 1, kind: "chapter", head_rev: 0,
    }))] }));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await treeRequested;
    const drawer = page.locator("#manuscript-drawer");
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const lore = drawer.getByRole("button", { name: "Mira · character", exact: true });
    const exportLink = drawer.getByRole("link", { name: "Export local chapter", exact: true });
    await expect(drawer.getByRole("status")).toHaveText("Loading manuscript…");
    await expect(lore).toBeInViewport();
    await expect(exportLink).toBeInViewport();
    const before = { outline: await outline.boundingBox(), lore: await lore.boundingBox(), export: await exportLink.boundingBox() };
    expect(before.outline).not.toBeNull();
    expect(before.lore).not.toBeNull();
    expect(before.export).not.toBeNull();
    await drawer.screenshot({ path: `../target/guz-820/${testInfo.project.name}-loading-${width}.png` });
    await lore.click();
    const editForm = drawer.getByRole("dialog", { name: "Edit lore", exact: true });
    await expect(editForm).toBeVisible();
    await expect(editForm.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Mira");
    await page.keyboard.press("Escape");
    await expect(editForm).toBeHidden();
    const downloadStarted = page.waitForEvent("download");
    await exportLink.click();
    expect((await downloadStarted).suggestedFilename()).toBe("chapter-local.json");
    await expect(drawer.getByRole("status")).toHaveText("Loading manuscript…");
    const interacted = { outline: await outline.boundingBox(), lore: await lore.boundingBox(), export: await exportLink.boundingBox() };
    expect(interacted).toEqual(before);
    release();
    const lastChapter = outline.getByRole("button", { name: "Outline chapter 150", exact: true });
    await expect(lastChapter).toBeAttached();
    await expect(drawer.getByText("Loading manuscript…", { exact: true })).toHaveCount(0);
    const after = { outline: await outline.boundingBox(), lore: await lore.boundingBox(), export: await exportLink.boundingBox() };
    console.log(JSON.stringify({ browser: testInfo.project.name, width, before, interacted, after }));
    expect(after).toEqual(before);
    const scroll = await outline.evaluate(element => ({ height: element.clientHeight, content: element.scrollHeight, overflow: getComputedStyle(element).overflowY }));
    expect(scroll.height).toBeLessThanOrEqual(320);
    expect(scroll.height).toBeGreaterThan(0);
    expect(scroll.content).toBeGreaterThan(scroll.height * 5);
    expect(scroll.overflow).toBe("auto");
    await lore.click();
    await expect(editForm).toBeVisible();
    await expect(exportLink).toBeInViewport();
    await drawer.screenshot({ path: `../target/guz-820/${testInfo.project.name}-loaded-${width}.png` });
    await outline.evaluate(element => { element.scrollTop = element.scrollHeight; });
    await expect(lastChapter).toBeInViewport();
    expect(await lore.boundingBox()).toEqual(before.lore);
    expect(await exportLink.boundingBox()).toEqual(before.export);
    expect(await drawer.evaluate(element => element.scrollTop)).toBe(0);
  });
}
