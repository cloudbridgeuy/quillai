import { expect, test, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function left(locator: Locator): Promise<number> {
  const box = await locator.boundingBox();
  if (!box) throw new Error("Element has no box");
  return box.x;
}

async function expectInsideOutline(outline: Locator, control: Locator) {
  await expect(control).toBeInViewport({ ratio: 1 });
  const area = await outline.boundingBox();
  const box = await control.boundingBox();
  if (!area || !box) throw new Error("Element has no box");
  expect(box.y).toBeGreaterThanOrEqual(area.y);
  expect(box.y + box.height).toBeLessThanOrEqual(area.y + area.height + 0.5);
}

async function openForm(page: Page, outline: Locator, button: Locator, shot: string) {
  await button.click();
  const form = outline.locator("[data-manuscript-draft]");
  await expect(form).toBeVisible();
  for (const control of [
    form.getByRole("textbox", { name: "Piece title", exact: true }),
    form.getByRole("combobox", { name: "Label", exact: true }),
    form.getByRole("combobox", { name: "Format", exact: true }),
    form.getByRole("button", { name: "Create", exact: true }),
    form.getByRole("button", { name: "Cancel", exact: true }),
  ]) await expectInsideOutline(outline, control);
  await page.locator("#manuscript-drawer").screenshot({ path: shot });
  await form.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(form).toHaveCount(0);
}

for (const width of [1100, 1440]) {
  test(`create forms fit the manuscript tree and align with their level at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const shots = `../target/guz-971/${testInfo.project.name}`;
    const prefix = `Fit ${testInfo.project.name} ${width}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status()).toBe(201);
      return response.json();
    };
    const universe = await post(`${api}/universes`, { title: prefix, position: 0 });
    const container = await post(`${api}/universes/${universe.id}/containers`, { title: `${prefix} shelf`, position: 0, kind: "collection" });
    await post(`${api}/containers/${container.id}/pieces`, { title: `${prefix} member`, position: 0, label: "short_story", shape: "leaf" });
    const loose = await post(`${api}/universes/${universe.id}/pieces`, { title: `${prefix} loose`, position: 0, label: "short_story", shape: "leaf" });

    await page.setViewportSize({ width, height: 900 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, editor: "1" })}`);
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const drawer = page.locator("#manuscript-drawer");
    await expect(drawer.getByText("Open a piece or chapter to manage its universe lore.", { exact: true })).toBeVisible();

    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, universeId: String(universe.id), pieceId: String(loose.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Manuscript", exact: true }).click();
    const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
    const universeItem = outline.locator("li").filter({ has: page.locator("summary", { hasText: new RegExp(`^${prefix}$`) }) }).last();
    const containerItem = universeItem.locator("li").filter({ has: page.locator("summary", { hasText: `${prefix} shelf` }) }).last();
    const member = containerItem.getByRole("button", { name: `${prefix} member`, exact: true });
    const addPiece = containerItem.getByRole("button", { name: "+ Piece", exact: true });
    const loosePiece = universeItem.getByRole("button", { name: `${prefix} loose`, exact: true });
    const addLoose = universeItem.getByRole("button", { name: "+ Loose piece", exact: true });
    const addContainer = universeItem.getByRole("button", { name: "+ Container", exact: true });
    await expect(loosePiece).toHaveAttribute("aria-pressed", "true");
    await expect(member).toBeVisible();

    expect(await left(addPiece)).toBeCloseTo(await left(member), 0);
    expect(await left(addLoose)).toBeCloseTo(await left(loosePiece), 0);
    expect(await left(addContainer)).toBeCloseTo(await left(loosePiece), 0);
    await drawer.screenshot({ path: `${shots}/tree-${width}.png` });

    await openForm(page, outline, addPiece, `${shots}/container-piece-form-${width}.png`);
    await openForm(page, outline, addLoose, `${shots}/loose-piece-form-${width}.png`);
  });
}
