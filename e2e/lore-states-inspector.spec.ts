import { expect, test, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("the context inspector shows the lore state applied for the piece's timeline point, else base", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const send = async (method: "post" | "put", client: APIRequestContext, url: string, data: object, status = 201) => {
    const response = await client[method](url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const universe = await send("post", request, `${api}/universes`, { title: "Lore states universe", position: 0 });
  const early = await send("post", request, `${api}/universes/${universe.id}/timeline`, { name: "Early", position: 10 });
  const late = await send("post", request, `${api}/universes/${universe.id}/timeline`, { name: "Late", position: 20 });
  const lore = await send("post", request, `${api}/universes/${universe.id}/lore`, {
    kind: "character", name: "Mira", visible_text: "BASE_TEXT", hidden_notes: "", pinned: true,
  });
  const states = `${api}/universes/${universe.id}/lore/${lore.id}/states`;
  await send("post", request, states, { name: "Young", pointId: early.id, visibleText: "YOUNG_TEXT" });
  await send("post", request, states, { name: "Old", pointId: late.id, visibleText: "OLD_TEXT" });
  const open = async (pointId: number | null) => {
    const piece = await send("post", request, `${api}/universes/${universe.id}/pieces`, { title: `Piece ${pointId}`, position: 0, label: "short_story", shape: "leaf" });
    if (pointId !== null) await send("put", request, `${api}/pieces/${piece.id}`, { timelinePointId: pointId }, 200);
    await send("post", request, `${api}/pieces/${piece.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Mira waited.\n" }] } }, 200);
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
    await expectEditorReady(page.locator(".ql-editor"));
    await page.getByRole("button", { name: "Context inspector", exact: true }).click();
    return page.locator("#context-drawer");
  };
  await expect(await open(early.id)).toContainText("Mira · state Young");
  await expect(await open(late.id)).toContainText("Mira · state Old");
  await expect(await open(null)).toContainText("Mira · base");
});
