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

  const picker = page.getByRole("combobox", { name: "This chapter happens at", exact: true });
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
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
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "This chapter happens at", exact: true }).locator("option:checked")).toHaveText("Opening");
});

test("a chosen point applies the lore state, deleting it falls back to base, and a failed save keeps the picker", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const send = async (method: "post" | "put", url: string, data: object, status = 201) => {
    const response = await request[method](url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const universe = await send("post", `${api}/universes`, { title: "Timeline states universe", position: 0 });
  const point = await send("post", `${api}/universes/${universe.id}/timeline`, { name: "Late", position: 10 });
  const lore = await send("post", `${api}/universes/${universe.id}/lore`, {
    kind: "character", name: "Mira", visible_text: "BASE_TEXT", hidden_notes: "", pinned: true,
  });
  await send("post", `${api}/universes/${universe.id}/lore/${lore.id}/states`, { name: "Old", pointId: point.id, visibleText: "OLD_TEXT" });
  const piece = await send("post", `${api}/universes/${universe.id}/pieces`, { title: "Piece", position: 0, label: "short_story", shape: "leaf" });
  await send("post", `${api}/pieces/${piece.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Mira waited.\n" }] } }, 200);
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const inspector = page.locator("#context-drawer");
  const picker = page.getByRole("combobox", { name: "This chapter happens at", exact: true });

  await expect(inspector).toContainText("Mira · base");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await picker.selectOption({ label: "Late" });
  await expect(picker.locator("option:checked")).toHaveText("Late");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(inspector).toContainText("Mira · state Old");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await picker.selectOption({ label: "No point (base text)" });
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(inspector).toContainText("Mira · base");

  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await picker.selectOption({ label: "Late" });
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  await expect(inspector).toContainText("Mira · state Old");
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Timeline", exact: true });
  await dialog.getByRole("button", { name: "Late", exact: true }).click();
  await dialog.getByRole("button", { name: "Delete point", exact: true }).click();
  await dialog.getByRole("button", { name: "Confirm delete", exact: true }).click();
  await expect(dialog.getByText("No timeline points yet.")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(picker.locator("option:checked")).toHaveText("No point (base text)");

  const other = await send("post", `${api}/universes/${universe.id}/timeline`, { name: "Later", position: 20 });
  await page.reload();
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const reloaded = page.getByRole("combobox", { name: "This chapter happens at", exact: true });
  await page.route(`**/pieces/${piece.id}`, route => route.request().method() === "PUT"
    ? route.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify({ error: "boom", code: "INTERNAL" }) })
    : route.continue());
  await reloaded.selectOption({ label: other.name });
  await expect(page.getByRole("alert").filter({ hasText: /\S/ }).first()).toBeVisible();
  await expect(reloaded.locator("option:checked")).toHaveText("No point (base text)");
});

test("a chapter without its own point shows the parent's point as inherited", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const send = async (method: "post" | "put", url: string, data: object, status = 201) => {
    const response = await request[method](url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const universe = await send("post", `${api}/universes`, { title: "Inherit universe", position: 0 });
  const point = await send("post", `${api}/universes/${universe.id}/timeline`, { name: "Act one", position: 0 });
  const group = await send("post", `${api}/universes/${universe.id}/pieces`, { title: "Novel", position: 0, label: "novel", shape: "group" });
  await send("put", `${api}/pieces/${group.id}`, { timelinePointId: point.id }, 200);
  const child = await send("post", `${api}/pieces/${group.id}/children`, { title: "Chapter", position: 0 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(child.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();

  const picker = page.getByRole("combobox", { name: "This chapter happens at", exact: true });
  await expect(picker).toHaveValue("");
  await expect(picker.locator("option").first()).toHaveText("Inherit: Act one");
  await picker.selectOption({ label: "Act one" });
  await expect(picker.locator("option:checked")).toHaveText("Act one");
  await expect(picker.locator("option").first()).toHaveText("No point (base text)");
});
