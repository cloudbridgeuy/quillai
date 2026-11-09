import { expect, test, type Locator } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

function summaryTitles(scope: Locator): Promise<string[]> {
  return scope.locator("summary").evaluateAll(rows => rows.map(row => Array.from(row.childNodes)
    .filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent).join("").trim()));
}

for (const width of [1100, 1440]) {
  test(`create universe, book series, novel and chapters and save before opening at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors: string[] = [];
    const streamCloses: string[] = [];
    const posts: string[] = [];
    page.on("response", response => {
      if (response.url().startsWith(api) && response.status() >= 400) errors.push(`${response.status()} ${response.url()}`);
    });
    page.on("pageerror", error => errors.push(error.message));
    page.on("requestfailed", req => {
      const error = req.failure()?.errorText;
      const cancelledCatalog = req.url() === `${api}/ai/models` && (error === "net::ERR_ABORTED" || error === "cancelled");
      const streamClose = (error === "net::ERR_ABORTED" || error === "cancelled") && req.url().startsWith(api)
        && /^\/series\/\d+\/books\/\d+\/chapters\/\d+\/events$/.test(req.url().slice(api.length));
      if (streamClose) streamCloses.push(req.url());
      else if (req.url().startsWith(api) && !cancelledCatalog) errors.push(`${error} ${req.url()}`);
    });
    page.on("request", req => {
      if (req.url().startsWith(api) && req.method() === "POST") posts.push(req.url());
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
    await expect(page.getByRole("button", { name: "Manuscript", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expect(outline.getByRole("button", { name: "+ Universe", exact: true })).toBeEnabled();
    const existing = await (await request.get(`${api}/universes`)).json();
    const maximum = existing.reduce((max: number, item: { position: number }) => Math.max(max, item.position), -1);
    const title = `Created ${testInfo.project.name} ${width}`;
    await outline.getByRole("button", { name: "+ Universe", exact: true }).click();
    const universeInput = outline.getByRole("textbox", { name: "Universe title", exact: true });
    const beforeCancel = [...posts];
    await universeInput.fill("Cancelled");
    await universeInput.press("Escape");
    await expect(universeInput).toHaveCount(0);
    await expect(outline).toBeVisible();
    expect(posts).toEqual(beforeCancel);
    await outline.getByRole("button", { name: "+ Universe", exact: true }).click();
    await universeInput.fill("   ");
    await universeInput.press("Enter");
    await expect(outline.getByRole("alert")).toHaveText("Enter a title.");
    expect(posts).toEqual(beforeCancel);
    await universeInput.fill(`  ${title}  `);
    await universeInput.press("Enter");
    const universeRow = outline.locator("li").filter({ has: page.locator("summary", { hasText: title }) }).first();
    await expect(universeRow.locator("summary").first()).toHaveText(title);
    const universes = await (await request.get(`${api}/universes`)).json();
    expect(universes.at(-1)).toMatchObject({ title, position: maximum + 1 });
    await expect(outline.locator(".editor-manuscript-tree > ul > li > details[open] > summary").filter({ hasText: title })).toHaveCount(1);
    await universeRow.getByRole("button", { name: "+ Container", exact: true }).click();
    await expect(outline.getByRole("combobox", { name: "Kind", exact: true })).toHaveValue("book_series");
    await outline.getByRole("textbox", { name: "Container title", exact: true }).fill(title);
    await outline.getByRole("textbox", { name: "Container title", exact: true }).press("Enter");
    const seriesRow = universeRow.locator("li").filter({ has: page.getByRole("button", { name: `Rename '${title}'`, exact: true }) }).first();
    await expect.poll(() => summaryTitles(seriesRow)).toEqual([`${title} · Book series`]);
    const [series] = await (await request.get(`${api}/universes/${universes.at(-1).id}/containers`)).json();
    expect(series).toMatchObject({ title, kind: "book_series", position: 0 });
    expect(await (await request.get(`${api}/series/${series.id}`)).json()).toMatchObject({ id: series.id, title, position: 0 });
    const base = `${api}/series/${series.id}/books`;
    await seriesRow.getByRole("button", { name: "+ Piece", exact: true }).click();
    await expect(outline.getByRole("combobox", { name: "Label", exact: true })).toHaveValue("novel");
    await outline.getByRole("textbox", { name: "Piece title", exact: true }).fill("New book");
    await outline.getByRole("textbox", { name: "Piece title", exact: true }).press("Enter");
    await expect.poll(() => summaryTitles(seriesRow)).toEqual([`${title} · Book series`, "New book"]);
    const books = await (await request.get(base)).json();
    expect(books).toMatchObject([{ title: "New book", position: 0 }]);
    const chaptersUrl = `${base}/${books[0].id}/chapters`;
    for (const chapterTitle of ["First chapter", "Second chapter"]) {
      await seriesRow.getByRole("button", { name: "+ Chapter", exact: true }).click();
      const field = outline.getByRole("textbox", { name: "Chapter title", exact: true });
      await field.fill(chapterTitle);
      if (chapterTitle === "Second chapter") {
        await page.locator(".ql-editor").fill("Text saved before switching.");
      }
      await field.press("Enter");
      await expect(page.locator(".editor-chapter-title")).toHaveText(chapterTitle);
      await expectEditorReady(page.locator(".ql-editor"));
      if (chapterTitle === "First chapter") {
        if (!await outline.isVisible()) await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      }
    }
    if (!await outline.isVisible()) await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect(seriesRow.getByRole("button", { name: /^(First|Second) chapter$/ })).toHaveText(["First chapter", "Second chapter"]);
    await expect(seriesRow.getByRole("button", { name: "Second chapter", exact: true })).toHaveAttribute("aria-pressed", "true");
    const chapters = await (await request.get(chaptersUrl)).json();
    expect(chapters).toMatchObject([
      { title: "First chapter", position: 0, kind: "chapter" },
      { title: "Second chapter", position: 1, kind: "chapter" },
    ]);
    const saved = await (await request.get(`${chaptersUrl}/${chapters[0].id}`)).json();
    expect(saved.content.ops).toEqual([{ insert: "Text saved before switching.\n" }]);
    await expect(page.locator(".ql-editor")).toHaveText("");
    expect(posts.filter(url => url === chaptersUrl)).toHaveLength(2);
    await page.locator(".ql-editor").fill("New chapter survives reload.");
    await expect.poll(async () => {
      const response = await request.get(`${chaptersUrl}/${chapters[1].id}`);
      expect(response.status()).toBe(200);
      return (await response.json()).content.ops;
    }).toEqual([{ insert: "New chapter survives reload.\n" }]);
    await page.reload();
    await expectEditorReady(page.locator(".ql-editor"));
    await expect(page.locator(".editor-chapter-title")).toHaveText("Second chapter");
    await expect(page.locator(".ql-editor")).toHaveText("New chapter survives reload.");
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    await expect.poll(() => summaryTitles(seriesRow)).toEqual([`${title} · Book series`, "New book"]);
    await expect(seriesRow.getByRole("button", { name: /^(First|Second) chapter$/ })).toHaveText(["First chapter", "Second chapter"]);
    await expect(seriesRow.getByRole("button", { name: "Second chapter", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(outline.getByRole("button", { name: "+ Universe", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "+ Piece", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "+ Chapter", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "Second chapter", exact: true })).toBeInViewport();
    await expect(seriesRow.getByRole("button", { name: "+ Piece", exact: true })).toBeInViewport();
    await expect(seriesRow.getByRole("button", { name: "+ Chapter", exact: true })).toBeInViewport();
    expect(errors).toEqual([]);
    expect(new Set(streamCloses).size).toBe(streamCloses.length);
    await page.screenshot({ path: `../target/guz-891/${testInfo.project.name}/create-${width}.png` });
  });
}
