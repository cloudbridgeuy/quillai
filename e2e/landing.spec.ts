import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

function universeRow(page: Page, title: string) {
  return page.getByRole("region", { name: "Library", exact: true }).locator("li")
    .filter({ has: page.locator("summary", { hasText: new RegExp(`^${title}$`) }) }).last();
}

function universeDetails(page: Page, title: string) {
  return universeRow(page, title).locator(":scope > details");
}

function seeder(request: APIRequestContext, api: string) {
  return async (url: string, data: object) => {
    const response = await request.post(`${api}${url}`, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
}

function apiErrors(page: Page, api: string, allowed: string[] = []) {
  const errors: string[] = [];
  page.on("response", response => {
    const entry = `${response.status()} ${response.url()}`;
    if (response.url().startsWith(api) && response.status() >= 400 && !allowed.includes(entry)) errors.push(entry);
  });
  page.on("pageerror", error => errors.push(error.message));
  return errors;
}

const emptyResume = "Nothing edited in this browser yet. Open or create a piece below.";

for (const width of [1100, 1440]) {
  test(`landing shows collapsed universe cards, a resume card, creates and returns from the editor at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors = apiErrors(page, api);
    const prefix = `Landing ${testInfo.project.name} ${width}`;
    const post = seeder(request, api);
    const universes = [];
    for (const [index, name] of ["Harbor", "Orchard", "Lantern"].entries()) {
      const universe = await post("/universes", { title: `${prefix} ${name}`, position: index });
      const container = await post(`/universes/${universe.id}/containers`, { title: `${prefix} ${name} shelf`, position: 0, kind: "collection" });
      await post(`/containers/${container.id}/pieces`, { title: `${prefix} ${name} member`, position: 0, label: "short_story", shape: "leaf" });
      await post(`/universes/${universe.id}/pieces`, { title: `${prefix} ${name} sketch`, position: 0, label: "short_story", shape: "leaf" });
      universes.push({ universe, container });
    }
    const [harbor] = universes;
    const edited = await post(`/containers/${harbor.container.id}/pieces`, { title: `${prefix} edited`, position: 1, label: "short_story", shape: "leaf" });
    await post(`/universes/${harbor.universe.id}/pieces`, { title: `${prefix} opened`, position: 1, label: "short_story", shape: "leaf" });

    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const library = page.getByRole("region", { name: "Library", exact: true });
    const resume = page.getByRole("region", { name: "Resume", exact: true });
    const harborName = `${prefix} Harbor`;
    const row = universeRow(page, harborName);
    const editor = page.locator(".ql-editor");
    const title = page.locator(".editor-chapter-title");
    const back = page.getByRole("button", { name: "Library", exact: true });
    await expect(library).toBeVisible();
    await expect(resume.getByText(emptyResume, { exact: true })).toBeVisible();
    await expect(resume.getByRole("button", { name: "Open", exact: true })).toHaveCount(0);
    for (const name of ["Harbor", "Orchard", "Lantern"]) {
      await expect(universeDetails(page, `${prefix} ${name}`)).not.toHaveAttribute("open");
      await expect(universeRow(page, `${prefix} ${name}`).getByRole("button", { name: `${prefix} ${name} sketch`, exact: true })).toBeHidden();
    }
    const card = await row.boundingBox();
    expect(card && card.width < width / 2).toBe(true);
    await expect(editor).toHaveCount(0);

    await row.locator(":scope > details > summary").click();
    await expect(universeDetails(page, harborName)).toHaveAttribute("open");
    const grid = await library.locator(".editor-manuscript-tree > ul").boundingBox();
    const open = await row.boundingBox();
    expect(grid && open && Math.abs(grid.width - open.width) < 2).toBe(true);
    await expect(row.locator("summary", { hasText: `${prefix} Harbor shelf · Collection` })).toBeVisible();
    await expect(row.getByRole("button", { name: `${prefix} Harbor member`, exact: true })).toBeVisible();
    await row.getByRole("button", { name: `${prefix} edited`, exact: true }).click();
    await expect(title).toHaveText(`${prefix} edited`);
    await expectEditorReady(editor);
    await editor.click();
    await page.keyboard.type("Resume me.");
    await back.click();
    await expect(library).toBeVisible();
    expect(new URL(page.url()).searchParams.has("pieceId")).toBe(false);
    expect((await (await request.get(`${api}/pieces/${edited.id}`)).json()).content.ops.map((op: { insert: string }) => op.insert).join(""))
      .toContain("Resume me.");

    await expect(universeDetails(page, harborName)).toHaveAttribute("open");
    await expect(universeDetails(page, `${prefix} Orchard`)).not.toHaveAttribute("open");
    await row.getByRole("button", { name: `${prefix} opened`, exact: true }).click();
    await expect(title).toHaveText(`${prefix} opened`);
    await expectEditorReady(editor);
    await back.click();
    await expect(resume.getByText(`${prefix} edited`, { exact: true })).toBeVisible();
    await expect(resume.getByText(`${harborName} · ${prefix} Harbor shelf`, { exact: true })).toBeVisible();
    await expect(resume.getByText(emptyResume, { exact: true })).toHaveCount(0);
    await page.screenshot({ path: `../target/guz-978/${testInfo.project.name}/landing-${width}.png`, fullPage: true });
    await resume.getByRole("button", { name: "Open", exact: true }).click();
    await expect(title).toHaveText(`${prefix} edited`);
    await expectEditorReady(editor);
    await expect(editor).toContainText("Resume me.");
    await back.click();

    await row.getByRole("button", { name: "+ Loose piece", exact: true }).click();
    await library.getByRole("textbox", { name: "Piece title", exact: true }).fill(`${prefix} created`);
    await library.getByRole("textbox", { name: "Piece title", exact: true }).press("Enter");
    await expect(title).toHaveText(`${prefix} created`);
    await expectEditorReady(editor);
    await back.click();
    await expect(library).toBeVisible();
    await page.reload();
    await expect(row.getByRole("button", { name: `${prefix} created`, exact: true })).toBeVisible();
    const loose = await (await request.get(`${api}/universes/${harbor.universe.id}/pieces?container=none`)).json();
    expect(loose.map((piece: { title: string }) => piece.title)).toContain(`${prefix} created`);
    expect(errors).toEqual([]);
  });

  test(`landing resume handles a deleted leaf without opening the editor at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const prefix = `Landing gone ${testInfo.project.name} ${width}`;
    const post = seeder(request, api);
    const universe = await post("/universes", { title: prefix, position: 0 });
    const piece = await post(`/universes/${universe.id}/pieces`, { title: `${prefix} piece`, position: 0, label: "short_story", shape: "leaf" });
    const errors = apiErrors(page, api, [`404 ${api}/pieces/${piece.id}`]);

    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const resume = page.getByRole("region", { name: "Resume", exact: true });
    const editor = page.locator(".ql-editor");
    await universeRow(page, prefix).locator(":scope > details > summary").click();
    await universeRow(page, prefix).getByRole("button", { name: `${prefix} piece`, exact: true }).click();
    await expectEditorReady(editor);
    await editor.click();
    await page.keyboard.type("Gone soon.");
    await page.getByRole("button", { name: "Library", exact: true }).click();
    await expect(resume.getByText(`${prefix} piece`, { exact: true })).toBeVisible();

    expect((await request.delete(`${api}/pieces/${piece.id}`)).status()).toBe(204);
    await page.reload();
    await expect(resume.getByText(`${prefix} piece is no longer in the library.`, { exact: true })).toBeVisible();
    await expect(resume.getByRole("button", { name: "Open", exact: true })).toHaveCount(0);
    await expect(editor).toHaveCount(0);
    expect(new URL(page.url()).searchParams.has("pieceId")).toBe(false);
    await resume.getByRole("button", { name: "Forget", exact: true }).click();
    await expect(resume.getByText(emptyResume, { exact: true })).toBeVisible();
    await page.reload();
    await expect(resume.getByText(emptyResume, { exact: true })).toBeVisible();
    expect(errors).toEqual([]);
  });

  test(`landing resumes a chapter-scope leaf at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors = apiErrors(page, api);
    const prefix = `Landing chapter ${testInfo.project.name} ${width}`;
    const post = seeder(request, api);
    const series = await post("/series", { title: `${prefix} series`, position: 0 });
    const book = await post(`/series/${series.id}/books`, { title: `${prefix} book`, position: 0 });
    const chapter = await post(`/series/${series.id}/books/${book.id}/chapters`, { title: `${prefix} chapter`, position: 0, kind: "chapter" });
    const { universeId } = await (await request.get(`${api}/containers/${series.id}`)).json();
    const universe = await (await request.get(`${api}/universes/${universeId}`)).json();
    const scope = { seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) };

    await page.setViewportSize({ width, height: 1000 });
    const editor = page.locator(".ql-editor");
    const title = page.locator(".editor-chapter-title");
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, ...scope })}`);
    await expect(title).toHaveText(`${prefix} chapter`);
    await expectEditorReady(editor);
    await editor.click();
    await page.keyboard.type("Chapter resume.");
    await page.getByRole("button", { name: "Library", exact: true }).click();

    const resume = page.getByRole("region", { name: "Resume", exact: true });
    await expect(resume.getByText(`${prefix} chapter`, { exact: true })).toBeVisible();
    await expect(resume.getByText(`${universe.title} · ${prefix} series · ${prefix} book`, { exact: true })).toBeVisible();
    await expect(universeDetails(page, universe.title)).toHaveAttribute("open");
    await resume.getByRole("button", { name: "Open", exact: true }).click();
    await expect(title).toHaveText(`${prefix} chapter`);
    await expectEditorReady(editor);
    await expect(editor).toContainText("Chapter resume.");
    expect(new URL(page.url()).searchParams.get("chapterId")).toBe(scope.chapterId);
    expect(errors).toEqual([]);
  });
}
