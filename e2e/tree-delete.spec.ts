import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

type Kind = "chapter" | "piece" | "book" | "series" | "collection" | "universe";

async function seed(request: APIRequestContext, api: string, title: string) {
  const post = async (path: string, data: object) => {
    const response = await request.post(`${api}${path}`, { data });
    expect(response.status(), await response.text()).toBe(201);
    return response.json();
  };
  const series = await post("/series", { title: `${title} series`, position: 0 });
  const book = await post(`/series/${series.id}/books`, { title: `${title} book`, position: 0 });
  const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, {
    title: `${title} chapter`, position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] },
  });
  const container = await (await request.get(`${api}/containers/${series.id}`)).json();
  const universeResponse = await request.put(`${api}/universes/${container.universeId}`, { data: { title: `${title} universe` } });
  expect(universeResponse.status(), await universeResponse.text()).toBe(200);
  const universe = await universeResponse.json();
  const piece = await post(`/universes/${universe.id}/pieces`, { title: `${title} piece`, position: 0, label: "short_story", shape: "leaf" });
  const collection = await post(`/universes/${universe.id}/containers`, { title: `${title} collection`, position: 1, kind: "collection" });
  return {
    titles: { chapter: chapter.title, piece: piece.title, book: book.title, series: series.title, collection: collection.title, universe: universe.title },
    urls: {
      chapter: `${api}/series/${series.id}/books/${book.id}/chapters/${chapter.id}`,
      piece: `${api}/pieces/${piece.id}`, book: `${api}/series/${series.id}/books/${book.id}`,
      series: `${api}/series/${series.id}`, collection: `${api}/containers/${collection.id}`,
      universe: `${api}/universes/${universe.id}`,
    },
    scope: { seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) },
  };
}

function row(tree: Locator, title: string, kind: Kind) {
  if (kind === "chapter" || kind === "piece") {
    return tree.getByRole("button", { name: title, exact: true }).locator("xpath=ancestor::li[1]");
  }
  return tree.locator("summary").filter({ hasText: title });
}

function control(tree: Locator, title: string, kind: Kind) {
  const escapedTitle = title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return row(tree, title, kind).getByRole("button", { name: new RegExp(`^Delete '${escapedTitle}'$`), includeHidden: true });
}

async function expandUniverse(tree: Locator, title: string) {
  const summary = row(tree, title, "universe");
  await expect(summary).toBeVisible();
  if (await summary.locator("..").getAttribute("open") === null) await summary.click();
}

async function openLanding(page: Page, api: string, universe: string) {
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
  const tree = page.getByRole("region", { name: "Library", exact: true });
  await expandUniverse(tree, universe);
  return tree;
}

async function revealed(locator: Locator) {
  return locator.evaluate(element => {
    for (let current: Element | null = element; current; current = current.parentElement) {
      const style = getComputedStyle(current);
      if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse" || Number(style.opacity) === 0) return false;
    }
    return element.getClientRects().length > 0;
  });
}

for (const width of [1100, 1440]) {
  for (const kind of ["chapter", "piece", "book", "series", "collection", "universe"] as const) {
    test(`${kind} delete control responds to hover and keyboard focus at ${width}px`, async ({ page, request }, testInfo) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const data = await seed(request, api, `Visibility ${testInfo.project.name} ${width} ${kind}`);
      await page.setViewportSize({ width, height: 1000 });
      const tree = await openLanding(page, api, data.titles.universe);
      const item = row(tree, data.titles[kind], kind);
      const button = control(tree, data.titles[kind], kind);
      await expect(item).toBeVisible();
      await expect(button).toHaveCount(1, { timeout: 2500 });
      await page.mouse.move(0, 0);
      await expect.poll(() => revealed(button)).toBe(false);
      await item.hover();
      await expect.poll(() => revealed(button)).toBe(true);
      await expect(button).toBeVisible();
      const bounds = await button.boundingBox();
      const rowBounds = await item.boundingBox();
      expect(bounds && rowBounds && bounds.width <= 40 && bounds.height <= 40 && bounds.x + bounds.width > rowBounds.x + rowBounds.width / 2).toBe(true);
      await page.mouse.move(0, 0);
      const title = kind === "chapter" || kind === "piece" ? item.getByRole("button", { name: data.titles[kind], exact: true }) : item;
      await title.focus();
      await page.keyboard.press("Tab");
      await expect(button).toBeFocused();
      await expect.poll(() => revealed(button)).toBe(true);
    });

    test(`${kind} deletes, undoes, and appears in recently deleted at ${width}px`, async ({ page, request }, testInfo) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const data = await seed(request, api, `Removal ${testInfo.project.name} ${width} ${kind}`);
      await page.setViewportSize({ width, height: 1000 });
      const tree = await openLanding(page, api, data.titles.universe);
      const item = row(tree, data.titles[kind], kind);
      const button = control(tree, data.titles[kind], kind);
      await expect(item).toBeVisible();
      await expect(button).toHaveCount(1, { timeout: 2500 });
      const remove = async () => {
        await item.hover();
        const response = page.waitForResponse(response => response.url() === data.urls[kind] && response.request().method() === "DELETE");
        await button.click();
        expect((await response).status()).toBe(204);
        await expect(item).toHaveCount(0);
        const alert = page.getByRole("status").filter({ hasText: `Deleted '${data.titles[kind]}'.` });
        await expect(alert).toContainText(`Deleted '${data.titles[kind]}'.`);
        await expect(alert.getByRole("button", { name: "Undo", exact: true })).toBeVisible();
        await expect(alert.getByRole("link", { name: "Recently deleted", exact: true }).or(alert.getByRole("button", { name: "Recently deleted", exact: true }))).toBeVisible();
        return alert;
      };
      let alert = await remove();
      const restore = page.waitForResponse(response => response.url() === `${data.urls[kind]}/restore` && response.request().method() === "POST");
      await alert.getByRole("button", { name: "Undo", exact: true }).click();
      expect((await restore).status()).toBe(200);
      if (kind !== "universe") await expandUniverse(tree, data.titles.universe);
      await expect(item).toBeVisible();
      alert = await remove();
      await alert.getByRole("link", { name: "Recently deleted", exact: true }).or(alert.getByRole("button", { name: "Recently deleted", exact: true })).click();
      await expect(page.getByRole("region", { name: "Library", exact: true })).toBeVisible();
      await expect(page.getByRole("region", { name: "Recently deleted", exact: true }).getByText(data.titles[kind], { exact: true })).toBeVisible();
    });
  }

  test(`deleting the open chapter waits for pending saves at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const data = await seed(request, api, `Pending ${testInfo.project.name} ${width}`);
    const chapterUrl = data.urls.chapter;
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, ...data.scope })}`);
    const editor = page.locator(".ql-editor");
    await expectEditorReady(editor);
    let release!: () => void;
    const gate = new Promise<void>(resolve => { release = resolve; });
    let sawPost!: () => void;
    const held = new Promise<void>(resolve => { sawPost = resolve; });
    let deletes = 0;
    page.on("request", request => { if (request.url() === chapterUrl && request.method() === "DELETE") deletes++; });
    await page.route(`${chapterUrl}/deltas`, async route => {
      if (route.request().method() !== "POST") return route.continue();
      sawPost();
      await gate;
      await route.continue();
    });
    try {
      await editor.click();
      await page.keyboard.press("ControlOrMeta+End");
      await page.keyboard.insertText(" first flight");
      await held;
      await page.keyboard.insertText(" pending text");
      await expect(editor).toHaveText("Opening line first flight pending text\n");
      await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      const tree = page.getByRole("region", { name: "Manuscript outline", exact: true });
      const item = row(tree, data.titles.chapter, "chapter");
      const button = control(tree, data.titles.chapter, "chapter");
      await expect(item).toBeVisible();
      await expect(button).toHaveCount(1, { timeout: 2500 });
      await item.hover();
      await button.click();
      await page.waitForTimeout(500);
      expect(deletes).toBe(0);
      expect(new URL(page.url()).searchParams.get("chapterId")).toBe(data.scope.chapterId);
      await expect(editor).toBeVisible();
      release();
      await expect(page.getByRole("region", { name: "Library", exact: true })).toBeVisible();
      expect(deletes).toBe(1);
      expect((await request.post(`${chapterUrl}/restore`)).status()).toBe(200);
      const restored = await (await request.get(chapterUrl)).json();
      expect(restored.content.ops.map((op: { insert: string }) => op.insert).join("")).toBe("Opening line first flight pending text\n");
    } finally {
      release();
      await page.unrouteAll({ behavior: "wait" });
    }
  });
}
