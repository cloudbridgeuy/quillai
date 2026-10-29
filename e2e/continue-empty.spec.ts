import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

function snapshot(element: Element, selection?: { index: number; length: number }) {
  const host = element.closest(".editor-host")!;
  const key = Object.keys(host).find(key => key.startsWith("__reactFiber$"))!;
  type Fiber = { ref?: { current?: Quill }; return?: Fiber };
  let fiber: Fiber | undefined = (host as unknown as Record<string, Fiber>)[key];
  while (fiber) {
    const quill = fiber.ref?.current;
    if (quill?.root === element) {
      if (selection) quill.setSelection(selection.index, selection.length, "user");
      return { text: quill.getText(), selection: quill.getSelection() };
    }
    fiber = fiber.return;
  }
  throw new Error("Quill instance not found");
}

test("empty Continue reports length and stop without changing chapter, selection or thread", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identities = Object.fromEntries(["seed", "length", "stop"].map(kind => [kind, `GUZ866-${kind}-${crypto.randomUUID()}`]));
  const text = `Opening ${Object.values(identities).join(" ")}\n`;
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(status);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Empty Continue", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Empty Continue", position: 0 });
  const chapters = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await post(chapters, { title: "Empty Continue", position: 0, kind: "chapter", content: { ops: [{ insert: text }] } });
  const chapterUrl = `${chapters}/${chapter.id}`;
  const roots = await (await request.get(`${api}/containers/${series.id}/pieces`)).json();
  const group = roots.find((piece: { title: string }) => piece.title === "Empty Continue");
  const groupDetail = await (await request.get(`${api}/pieces/${group.id}`)).json();
  const piece = groupDetail.children.find((child: { title: string }) => child.title === "Empty Continue");
  const pieceUrl = `${api}/pieces/${piece.id}`;
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  const run = async (kind: string) => {
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity: identities[kind], model: "e2e-continue" } })).status()).toBe(200);
    await page.getByRole("button", { name: /^Continue writing/ }).click();
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identities[kind]);
    return `${provider}/control/${upstream.id}`;
  };
  await editor.evaluate(snapshot, { index: 3, length: 0 });
  const seed = await run("seed");
  expect((await request.post(`${seed}/fragment`, { data: { text: "Seed prose." } })).status()).toBe(200);
  expect((await request.post(`${seed}/finish`, { data: { reason: "length" } })).status()).toBe(200);
  await expect(page.getByRole("button", { name: "Continue writing from a cut-off reply", exact: true })).toBeVisible();
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  const beforeChapterResponse = await request.get(chapterUrl);
  expect(beforeChapterResponse.status()).toBe(200);
  const beforeChapter = await beforeChapterResponse.json();
  const beforePieceResponse = await request.get(pieceUrl);
  expect(beforePieceResponse.status()).toBe(200);
  const beforePiece = await beforePieceResponse.json();
  const thread = async () => page.evaluate(() => {
    return JSON.parse(localStorage.getItem("quillai.editor.state.v1")!).threadState;
  });
  await expect.poll(async () => (await thread())?.thread?.messages?.length).toBeGreaterThan(0);
  const beforeThread = await thread();
  for (const [kind, message, selection] of [
    ["length", "The model used its whole budget without writing any text. Try again or pick another model.", { index: 3, length: 0 }],
    ["stop", "The model returned no text.", { index: 3, length: 4 }],
  ] as const) {
    const before = await editor.evaluate(snapshot, selection);
    const control = await run(kind);
    if (kind === "stop") await expect(page.getByRole("alert")).toHaveCount(0);
    if (kind === "length") expect((await request.post(`${control}/usage`)).status()).toBe(200);
    expect((await request.post(`${control}/finish`, { data: { reason: kind } })).status()).toBe(200);
    await expect.soft(page.getByRole("alert")).toHaveText(message);
    expect.soft(await editor.evaluate(snapshot)).toEqual(before);
    const chapterResponse = await request.get(chapterUrl);
    const pieceResponse = await request.get(pieceUrl);
    expect(chapterResponse.status()).toBe(200);
    expect(pieceResponse.status()).toBe(200);
    expect(await chapterResponse.json()).toEqual(beforeChapter);
    expect(await pieceResponse.json()).toEqual(beforePiece);
    expect.soft(await thread()).toEqual(beforeThread);
  }
  await editor.press("ArrowRight");
  await page.keyboard.type("x");
  await expect(page.getByRole("alert")).toHaveCount(0);
});
