import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

function universeRow(page: Page, title: string) {
  return page.getByRole("region", { name: "Library", exact: true }).locator("li")
    .filter({ has: page.locator("summary", { hasText: new RegExp(`^${title}$`) }) }).last();
}

for (const width of [1100, 1440]) {
  test(`landing lists the library, resumes, creates and returns from the editor at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const errors: string[] = [];
    page.on("response", response => {
      if (response.url().startsWith(api) && response.status() >= 400) errors.push(`${response.status()} ${response.url()}`);
    });
    page.on("pageerror", error => errors.push(error.message));
    const prefix = `Landing ${testInfo.project.name} ${width}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const universe = await post(`${api}/universes`, { title: prefix, position: 0 });
    const container = await post(`${api}/universes/${universe.id}/containers`, { title: `${prefix} shelf`, position: 0, kind: "collection" });
    await post(`${api}/containers/${container.id}/pieces`, { title: `${prefix} member`, position: 0, label: "short_story", shape: "leaf" });
    const edited = await post(`${api}/universes/${universe.id}/pieces`, { title: `${prefix} edited`, position: 0, label: "short_story", shape: "leaf" });
    await post(`${api}/universes/${universe.id}/pieces`, { title: `${prefix} opened`, position: 1, label: "short_story", shape: "leaf" });

    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const library = page.getByRole("region", { name: "Library", exact: true });
    const row = universeRow(page, prefix);
    const editor = page.locator(".ql-editor");
    const title = page.locator(".editor-chapter-title");
    const back = page.getByRole("button", { name: "Library", exact: true });
    await expect(library).toBeVisible();
    await expect(row.locator("summary", { hasText: `${prefix} shelf · Collection` })).toBeVisible();
    await expect(row.getByRole("button", { name: `${prefix} member`, exact: true })).toBeVisible();
    await expect(row.getByRole("button", { name: `${prefix} edited`, exact: true })).toBeVisible();
    await expect(row.getByRole("button", { name: `${prefix} opened`, exact: true })).toBeVisible();
    await expect(page.getByText("Nothing edited in this browser yet. Open or create a piece below.", { exact: true })).toBeVisible();
    await expect(editor).toHaveCount(0);
    await page.screenshot({ path: `../target/guz-894/${testInfo.project.name}/landing-${width}.png`, fullPage: true });

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

    await row.getByRole("button", { name: `${prefix} opened`, exact: true }).click();
    await expect(title).toHaveText(`${prefix} opened`);
    await expectEditorReady(editor);
    await back.click();
    const resume = page.getByRole("button", { name: `Resume ${prefix} edited`, exact: true });
    await expect(resume).toBeVisible();
    await resume.click();
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
    const loose = await (await request.get(`${api}/universes/${universe.id}/pieces?container=none`)).json();
    expect(loose.map((piece: { title: string }) => piece.title)).toContain(`${prefix} created`);
    expect(errors).toEqual([]);
  });
}
