import { expect, test, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

function row(scope: Locator, page: Page, summary: string): Locator {
  return scope.locator("li").filter({ has: page.locator("summary", { hasText: new RegExp(`^${summary}$`) }) }).last();
}

for (const width of [1100, 1440]) {
  test(`create universe, containers, pieces and children at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors: string[] = [];
    page.on("response", response => {
      if (response.url().startsWith(api) && response.status() >= 400) errors.push(`${response.status()} ${response.url()}`);
    });
    page.on("pageerror", error => errors.push(error.message));
    const shots = `../target/guz-941/${testInfo.project.name}`;
    const title = `Work ${testInfo.project.name} ${width}`;
    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
    const editor = page.locator(".ql-editor");
    const drawerButton = page.getByRole("button", { name: "Manuscript", exact: true });
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const openOutline = async () => {
      if (!await outline.isVisible()) await drawerButton.click();
      await expect(outline).toBeVisible();
    };
    const opened = async (name: string) => {
      await expect(page.locator(".editor-chapter-title")).toHaveText(name);
      await expectEditorReady(editor);
      await openOutline();
    };
    await expect(drawerButton).toBeVisible();
    await openOutline();

    await outline.getByRole("button", { name: "+ Universe", exact: true }).click();
    const universeTitle = outline.getByRole("textbox", { name: "Universe title", exact: true });
    await universeTitle.fill("Cancelled universe");
    await universeTitle.press("Escape");
    await expect(universeTitle).toHaveCount(0);
    await outline.getByRole("button", { name: "+ Universe", exact: true }).click();
    await universeTitle.fill(` ${title} `);
    await universeTitle.press("Enter");
    const universe = row(outline, page, title);
    await expect(universe.locator("summary").first()).toHaveText(title);

    await universe.getByRole("button", { name: "+ Loose piece", exact: true }).click();
    await outline.getByRole("textbox", { name: "Piece title", exact: true }).fill("Loose note");
    await expect(outline.getByRole("combobox", { name: "Label", exact: true })).toHaveValue("piece");
    await expect(outline.getByRole("combobox", { name: "Format", exact: true })).toHaveValue("prose");
    await outline.getByRole("textbox", { name: "Piece title", exact: true }).press("Enter");
    await opened("Loose note");
    await editor.click();
    await page.keyboard.type("Loose text.");
    await openOutline();

    const container = async (name: string, kind: string, kindLabel: string) => {
      await universe.getByRole("button", { name: "+ Container", exact: true }).click();
      await outline.getByRole("textbox", { name: "Container title", exact: true }).fill(name);
      await outline.getByRole("combobox", { name: "Kind", exact: true }).selectOption(kind);
      await outline.getByRole("button", { name: "Create", exact: true }).click();
      const created = row(universe, page, `${name} · ${kindLabel}`);
      await expect(created).toBeVisible();
      return created;
    };
    const rootPiece = async (parent: Locator, name: string, label?: string) => {
      await parent.getByRole("button", { name: "+ Piece", exact: true }).click();
      if (label) await outline.getByRole("combobox", { name: "Label", exact: true }).selectOption(label);
      await outline.getByRole("textbox", { name: "Piece title", exact: true }).fill(name);
      await outline.getByRole("textbox", { name: "Piece title", exact: true }).press("Enter");
    };
    const chapter = async (parent: Locator, name: string) => {
      await parent.getByRole("button", { name: "+ Chapter", exact: true }).click();
      await outline.getByRole("textbox", { name: "Chapter title", exact: true }).fill(name);
      await outline.getByRole("textbox", { name: "Chapter title", exact: true }).press("Enter");
      await opened(name);
    };

    const saga = await container("Saga", "book_series", "Book series");
    await saga.getByRole("button", { name: "+ Piece", exact: true }).click();
    await expect(outline.getByRole("combobox", { name: "Label", exact: true }).locator("option"))
      .toHaveText(["Novel", "Novella"]);
    await outline.getByRole("button", { name: "Cancel", exact: true }).click();
    await rootPiece(saga, "Saga novel");
    await expect(row(saga, page, "Saga novel")).toBeVisible();
    await chapter(row(saga, page, "Saga novel"), "Saga one");

    const anthology = await container("Anthology", "collection", "Collection");
    await rootPiece(anthology, "Anthology novel", "novel");
    await expect(row(anthology, page, "Anthology novel")).toBeVisible();
    await chapter(row(anthology, page, "Anthology novel"), "Anthology one");

    const show = await container("Show", "tv_series", "TV series");
    await show.getByRole("button", { name: "+ Piece", exact: true }).click();
    await outline.getByRole("textbox", { name: "Piece title", exact: true }).fill("Pilot");
    await expect(outline.getByRole("combobox", { name: "Label", exact: true })).toHaveValue("episode");
    await expect(outline.getByRole("combobox", { name: "Format", exact: true })).toHaveValue("script");
    await outline.getByRole("combobox", { name: "Format", exact: true }).selectOption("prose");
    await outline.getByRole("combobox", { name: "Format", exact: true }).selectOption("script");
    await page.locator("#manuscript-drawer").screenshot({ path: `${shots}/create-form-${width}.png` });
    await outline.getByRole("button", { name: "Create", exact: true }).click();
    await opened("Pilot");

    const universes = await (await request.get(`${api}/universes`)).json();
    const created = universes.find((item: { title: string }) => item.title === title);
    expect(created).toBeTruthy();
    const containers = await (await request.get(`${api}/universes/${created.id}/containers`)).json();
    expect(containers).toMatchObject([
      { title: "Saga", kind: "book_series", position: 0 },
      { title: "Anthology", kind: "collection", position: 1 },
      { title: "Show", kind: "tv_series", position: 2 },
    ]);
    const loose = await (await request.get(`${api}/universes/${created.id}/pieces?container=none`)).json();
    expect(loose).toMatchObject([{ title: "Loose note", label: "piece", format: "prose", shape: "leaf", position: 0 }]);
    await expect.poll(async () => (await (await request.get(`${api}/pieces/${loose[0].id}`)).json()).content.ops)
      .toEqual([{ insert: "Loose text.\n" }]);
    const roots = async (id: number) => (await request.get(`${api}/containers/${id}/pieces`)).json();
    const [sagaRoots, anthologyRoots, showRoots] = await Promise.all(containers.map((item: { id: number }) => roots(item.id)));
    expect(sagaRoots).toMatchObject([{ title: "Saga novel", label: "novel", format: "prose", shape: "group" }]);
    expect(anthologyRoots).toMatchObject([{ title: "Anthology novel", label: "novel", format: "prose", shape: "group" }]);
    expect(showRoots).toMatchObject([{ title: "Pilot", label: "episode", format: "script", shape: "leaf" }]);
    for (const [group, child] of [[sagaRoots[0], "Saga one"], [anthologyRoots[0], "Anthology one"]]) {
      const detail = await (await request.get(`${api}/pieces/${group.id}`)).json();
      expect(detail.children).toMatchObject([{ title: child, label: "chapter", format: "prose", shape: "leaf", position: 0 }]);
    }
    const books = await (await request.get(`${api}/series/${containers[0].id}/books`)).json();
    expect(books).toMatchObject([{ title: "Saga novel", position: 0 }]);

    await page.reload();
    await expectEditorReady(editor);
    await expect(page.locator(".editor-chapter-title")).toHaveText("Pilot");
    await openOutline();
    const reloaded = row(outline, page, title);
    for (const name of ["Loose note", "Saga one", "Anthology one", "Pilot"]) {
      await expect(reloaded.getByRole("button", { name, exact: true })).toBeVisible();
    }
    await expect(reloaded.locator("summary")).toHaveText([
      title, "Saga · Book series", "Saga novel", "Anthology · Collection", "Anthology novel", "Show · TV series",
    ]);
    await expect(reloaded.getByRole("button", { name: "Pilot", exact: true })).toHaveAttribute("aria-pressed", "true");
    await reloaded.getByRole("button", { name: "Loose note", exact: true }).click();
    await opened("Loose note");
    await expect(editor).toHaveText("Loose text.");
    await page.locator("#manuscript-drawer").screenshot({ path: `${shots}/create-tree-${width}.png` });
    expect(errors).toEqual([]);
  });
}
