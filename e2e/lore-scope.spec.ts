import { expect, test, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("pieces in a universe without a book series open and send that universe's lore at 1100px", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  const post = async (client: APIRequestContext, url: string, data: object, status = 201) => {
    const response = await client.post(url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const secret = "UNIVERSE_HIDDEN_NOTES_942";
  const universe = await post(request, `${api}/universes`, { title: "Lore scope universe", position: 0 });
  await post(request, `${api}/universes/${universe.id}/lore`, {
    kind: "character", name: "Mira", visible_text: "The ship's navigator.", hidden_notes: secret,
  });
  await post(request, `${api}/universes/${universe.id}/lore`, {
    kind: "place", name: "Harbor", visible_text: "The home port.", hidden_notes: secret, pinned: true,
  });
  const pieces = [];
  for (const title of ["Lore scope first piece", "Lore scope second piece"]) {
    const piece = await post(request, `${api}/universes/${universe.id}/pieces`, { title, position: pieces.length, label: "short_story", shape: "leaf" });
    await post(request, `${api}/pieces/${piece.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: `Mira waited in ${title}.\n` }] } }, 200);
    pieces.push(piece);
  }
  const containers = await (await request.get(`${api}/universes/${universe.id}/containers`)).json();
  expect(containers.filter((container: { kind: string }) => container.kind === "book_series")).toEqual([]);

  const bodies: string[] = [];
  await page.route("**/ai/generate", async route => {
    bodies.push(route.request().postData() ?? "");
    await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
  });
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(pieces[0].id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(page.locator(".editor-chapter-title")).toHaveText("Lore scope first piece");

  const continueOnce = async (count: number) => {
    await editor.click();
    await page.keyboard.press("Control+End");
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await expect.poll(() => bodies.length).toBe(count);
    await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeEnabled();
  };
  await continueOnce(1);

  await editor.click();
  await page.keyboard.press("Control+[");
  const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
  await expect(page.getByRole("region", { name: "Series lore", exact: true })).toContainText("Mira");
  await outline.getByRole("button", { name: "Lore scope second piece", exact: true }).click();
  await expect(page.locator(".editor-chapter-title")).toHaveText("Lore scope second piece");
  await expectEditorReady(editor);
  await editor.click();
  await page.keyboard.press("Control+[");
  await continueOnce(2);

  const expected = [
    { name: "Harbor", kind: "place", visible_text: "The home port.", level: "include" },
    { name: "Mira", kind: "character", visible_text: "The ship's navigator.", level: "include" },
  ];
  for (const body of bodies) {
    expect(body).not.toContain(secret);
    expect(JSON.parse(body).lore).toEqual(expected);
  }
  expect(errors).toEqual([]);
});
