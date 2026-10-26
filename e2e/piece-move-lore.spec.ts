import { expect, test, type APIRequestContext } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("placing a piece in another container of its universe keeps its text and Continue lore at 1100px", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  const post = async (client: APIRequestContext, url: string, data: object, status = 201) => {
    const response = await client.post(url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const universe = await post(request, `${api}/universes`, { title: "Move lore universe", position: 0 });
  await post(request, `${api}/universes/${universe.id}/lore`, {
    kind: "character", name: "Mira", visible_text: "The ship's navigator.", hidden_notes: "MOVE_HIDDEN_939",
  });
  await post(request, `${api}/universes/${universe.id}/lore`, {
    kind: "place", name: "Harbor", visible_text: "The home port.", pinned: true,
  });
  const containers = [];
  for (const title of ["Move first collection", "Move second collection"]) {
    containers.push(await post(request, `${api}/universes/${universe.id}/containers`, { title, position: containers.length, kind: "collection" }));
  }
  const text = "Mira waited by the moving piece.";
  const piece = await post(request, `${api}/containers/${containers[0].id}/pieces`, { title: "Moving piece", position: 0, label: "short_story", shape: "leaf" });
  await post(request, `${api}/pieces/${piece.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: `${text}\n` }] } }, 200);
  const before = await (await request.get(`${api}/pieces/${piece.id}`)).json();
  const loreBefore = await (await request.get(`${api}/universes/${universe.id}/lore`)).json();

  const bodies: string[] = [];
  await page.route("**/ai/generate", async route => {
    bodies.push(route.request().postData() ?? "");
    await route.fulfill({ contentType: "application/x-ndjson", body: `${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
  });
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(page.locator(".editor-chapter-title")).toHaveText("Moving piece");
  await expect(editor).toContainText(text);

  const continueOnce = async (count: number) => {
    await editor.click();
    await page.keyboard.press("Control+End");
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    await expect.poll(() => bodies.length).toBe(count);
    await expect(page.getByRole("button", { name: /^Continue writing/ })).toBeEnabled();
  };
  await continueOnce(1);
  const loreSent = JSON.parse(bodies[0]).lore;
  expect(loreSent).toEqual([
    { name: "Harbor", kind: "place", visible_text: "The home port.", level: "include" },
    { name: "Mira", kind: "character", visible_text: "The ship's navigator.", level: "include" },
  ]);

  const moved = await post(request, `${api}/pieces/${piece.id}/placement`, { containerId: containers[1].id, position: 0 }, 200);
  expect(moved).toMatchObject({ id: piece.id, containerId: containers[1].id, headRev: before.headRev, content: before.content });
  expect(await (await request.get(`${api}/containers/${containers[0].id}/pieces`)).json()).toEqual([]);

  await page.reload();
  await expectEditorReady(editor);
  await expect(page.locator(".editor-chapter-title")).toHaveText("Moving piece");
  await expect(editor).toContainText(text);
  await continueOnce(2);
  expect(JSON.parse(bodies[1]).lore).toEqual(loreSent);
  for (const body of bodies) expect(body).not.toContain("MOVE_HIDDEN_939");

  const after = await (await request.get(`${api}/pieces/${piece.id}`)).json();
  expect(after).toMatchObject({ id: piece.id, containerId: containers[1].id, headRev: before.headRev, content: before.content });
  expect(await (await request.get(`${api}/universes/${universe.id}/lore`)).json()).toEqual(loreBefore);
  expect(errors).toEqual([]);
});
