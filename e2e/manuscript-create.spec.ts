import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

for (const width of [1100, 1440]) {
  test(`create manuscript hierarchy and save before opening at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors: string[] = [];
    const posts: string[] = [];
    page.on("response", response => {
      if (response.url().startsWith(api) && response.status() >= 400) errors.push(`${response.status()} ${response.url()}`);
    });
    page.on("pageerror", error => errors.push(error.message));
    page.on("requestfailed", req => {
      const error = req.failure()?.errorText;
      const cancelledCatalog = req.url() === `${api}/ai/models` && error === "net::ERR_ABORTED";
      if (req.url().startsWith(api) && !cancelledCatalog) errors.push(`${error} ${req.url()}`);
    });
    page.on("request", req => {
      if (req.url().startsWith(api) && req.method() === "POST") posts.push(req.url());
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    await expect(page.getByRole("button", { name: "Manuscript", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    await expect(outline.getByRole("button", { name: "+ Series", exact: true })).toBeEnabled();
    const existing = await (await request.get(`${api}/series`)).json();
    const maximum = existing.reduce((max: number, item: { position: number }) => Math.max(max, item.position), -1);
    const title = `Created ${testInfo.project.name} ${width}`;
    await outline.getByRole("button", { name: "+ Series", exact: true }).click();
    const seriesInput = outline.getByRole("textbox", { name: "Series title", exact: true });
    const beforeCancel = [...posts];
    await seriesInput.fill("Cancelled");
    await seriesInput.press("Escape");
    await expect(seriesInput).toHaveCount(0);
    await expect(outline).toBeVisible();
    expect(posts).toEqual(beforeCancel);
    await outline.getByRole("button", { name: "+ Series", exact: true }).click();
    await seriesInput.fill("   ");
    await seriesInput.press("Enter");
    await expect(outline.getByRole("alert")).toHaveText("Enter a title.");
    expect(posts).toEqual(beforeCancel);
    await seriesInput.fill(`  ${title}  `);
    await seriesInput.press("Enter");
    const seriesRow = outline.locator("li").filter({ has: page.locator("summary", { hasText: title }) }).first();
    await expect(seriesRow.locator("summary").first()).toHaveText(title);
    const seriesRows = await (await request.get(`${api}/series`)).json();
    expect(seriesRows.at(-1)).toMatchObject({ title, position: maximum + 1 });
    const series = seriesRows.at(-1);
    await expect(outline.locator(".editor-manuscript-tree > ul > li > details > summary").last()).toHaveText(title);
    const base = `${api}/series/${series.id}/books`;
    await seriesRow.getByRole("button", { name: "+ Book", exact: true }).click();
    await outline.getByRole("textbox", { name: "Book title", exact: true }).fill("New book");
    await outline.getByRole("textbox", { name: "Book title", exact: true }).press("Enter");
    await expect(seriesRow.locator("summary")).toHaveText([title, "New book"]);
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
    await expect(seriesRow.locator("summary")).toHaveText([title, "New book"]);
    await expect(seriesRow.getByRole("button", { name: /^(First|Second) chapter$/ })).toHaveText(["First chapter", "Second chapter"]);
    await expect(seriesRow.getByRole("button", { name: "Second chapter", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(outline.getByRole("button", { name: "+ Series", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "+ Book", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "+ Chapter", exact: true })).toBeVisible();
    await expect(seriesRow.getByRole("button", { name: "Second chapter", exact: true })).toBeInViewport();
    await expect(seriesRow.getByRole("button", { name: "+ Book", exact: true })).toBeInViewport();
    await expect(seriesRow.getByRole("button", { name: "+ Chapter", exact: true })).toBeInViewport();
    expect(errors).toEqual([]);
    await page.screenshot({ path: `../target/guz-891/${testInfo.project.name}/create-${width}.png` });
  });
}
