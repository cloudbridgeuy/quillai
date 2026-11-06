import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("author adds, edits and deletes a lore state in the entry modal and sees the duplicate-point message", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const send = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const universe = await send(`${api}/universes`, { title: "Lore states modal universe", position: 0 });
  await send(`${api}/universes/${universe.id}/timeline`, { name: "Early", position: 10 });
  await send(`${api}/universes/${universe.id}/timeline`, { name: "Late", position: 20 });
  await send(`${api}/universes/${universe.id}/lore`, { kind: "character", name: "Mira", visible_text: "BASE_TEXT", hidden_notes: "" });
  const piece = await send(`${api}/universes/${universe.id}/pieces`, { title: "Piece", position: 0, label: "short_story", shape: "leaf" });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await page.getByRole("region", { name: "Series lore", exact: true }).getByRole("button", { name: /^Mira/ }).click();
  const dialog = page.getByRole("dialog", { name: "Edit lore", exact: true });
  const states = dialog.getByRole("region", { name: "States", exact: true });
  await expect(states.getByText("No states yet.", { exact: true })).toBeVisible();

  await states.getByLabel("State name", { exact: true }).fill("Young");
  await states.getByLabel("Point", { exact: true }).selectOption({ label: "Early" });
  await states.getByLabel("State text", { exact: true }).fill("YOUNG_TEXT");
  await states.getByRole("button", { name: "Add state", exact: true }).click();
  await expect(states.getByRole("button", { name: "Young · Early", exact: true })).toBeVisible();

  await states.getByLabel("State name", { exact: true }).fill("Duplicate");
  await states.getByLabel("Point", { exact: true }).selectOption({ label: "Early" });
  await states.getByRole("button", { name: "Add state", exact: true }).click();
  await expect(states.getByText("This entry already has a state at that point.", { exact: true })).toBeVisible();

  await states.getByRole("button", { name: "Young · Early", exact: true }).click();
  await states.getByLabel("Point", { exact: true }).selectOption({ label: "Late" });
  await states.getByRole("button", { name: "Save state", exact: true }).click();
  await expect(states.getByRole("button", { name: "Young · Late", exact: true })).toBeVisible();

  await states.getByRole("button", { name: "Young · Late", exact: true }).click();
  await states.getByRole("button", { name: "Delete state", exact: true }).click();
  await states.getByRole("button", { name: "Confirm delete", exact: true }).click();
  await expect(states.getByText("No states yet.", { exact: true })).toBeVisible();
  const listed = await (await request.get(`${api}/universes/${universe.id}/lore`)).json();
  expect(listed[0].states).toEqual([]);
});
