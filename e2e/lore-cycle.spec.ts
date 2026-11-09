import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("Lore in window cycles through matches on repeated clicks", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Cycle series", position: 0 } })).json();
  const base = `${api}/series/${series.id}`;
  const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
  const filler = (label: string, length = 40) => Array.from({ length }, (_, i) => `${label} line ${i}.\n`).join("");
  const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `${filler("z", 12)}Mira first.\n${filler("a")}Mira second.\n${filler("b")}Mira third.\n${filler("c", 10)}` }] },
  } })).json();
  expect((await request.post(`${base}/lore`, { data: { kind: "character", name: "Mira", visible_text: "The navigator.", level: "include" } })).status()).toBe(201);
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const list = page.getByRole("region", { name: "Lore in window", exact: true });
  await expect(list).toContainText("3 matches");
  const entry = list.getByRole("button", { name: /Mira/ });
  const card = page.getByRole("dialog", { name: "Lore match", exact: true });
  const tops: number[] = [];
  const matchTop = (index: number) => page.evaluate(i => {
    const nodes: Text[] = [];
    const walker = document.createTreeWalker(document.querySelector(".ql-editor")!, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) if ((node.textContent ?? "").startsWith("Mira")) nodes.push(node as Text);
    const range = document.createRange();
    range.setStart(nodes[i], 0);
    range.setEnd(nodes[i], 4);
    return range.getBoundingClientRect().top;
  }, index);
  for (const index of [0, 1, 2, 0, 1]) {
    await entry.click();
    await expect(card).toBeVisible();
    await expect.poll(async () => Math.abs((await matchTop(index)) - 350), { timeout: 5000 }).toBeLessThan(60);
    tops.push(index);
  }
  expect(tops).toEqual([0, 1, 2, 0, 1]);
  await page.keyboard.press("Escape");
  await expect(card).not.toBeVisible();
});
