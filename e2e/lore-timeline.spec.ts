import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("author adds timeline points and sets the chapter's point", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const send = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const universe = await send(`${api}/universes`, { title: "Timeline universe", position: 0 });
  const piece = await send(`${api}/universes/${universe.id}/pieces`, { title: "Piece", position: 0, label: "short_story", shape: "leaf" });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));

  const picker = page.getByLabel("This chapter happens at", { exact: true });
  await expect(picker).toHaveValue("");
  await expect(picker.locator("option").first()).toHaveText("No point (base text)");

  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Timeline", exact: true });
  for (const name of ["Prologue", "Act two"]) {
    await dialog.getByLabel("Point name", { exact: true }).fill(name);
    await dialog.getByRole("button", { name: "Add point", exact: true }).click();
    await expect(dialog.getByRole("button", { name, exact: true })).toBeVisible();
  }
  await dialog.getByRole("button", { name: "Move Act two up", exact: true }).click();
  await expect(dialog.getByRole("listitem").first()).toContainText("Act two");
  await dialog.getByRole("button", { name: "Prologue", exact: true }).click();
  await dialog.getByLabel("Point name", { exact: true }).fill("Opening");
  await dialog.getByRole("button", { name: "Rename point", exact: true }).click();
  await expect(dialog.getByRole("button", { name: "Opening", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");

  await picker.selectOption({ label: "Opening" });
  await expect(picker.locator("option:checked")).toHaveText("Opening");
  await page.reload();
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page.getByLabel("This chapter happens at", { exact: true }).locator("option:checked")).toHaveText("Opening");
});
