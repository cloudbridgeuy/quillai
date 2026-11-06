import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

test("lore entry generates an image and shows its thumbnail", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Image series", position: 0 } })).json();
  const base = `${api}/series/${series.id}`;
  const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Mira waits.\n" }] },
  } })).json();
  const mira = await (await request.post(`${base}/lore`, { data: { kind: "character", name: "Mira", visible_text: "The navigator." } })).json();
  const image = (id: number) => ({ id, loreEntryId: mira.id, timelinePointId: null, prompt: "p", model: "img1", size: "1024x1024",
    quality: "medium", mime: "image/webp", cost: 0.011, createdAt: "2026-11-06 10:00:00" });
  const posts: string[] = [];
  await page.route(`**/lore/${mira.id}/images`, route => {
    if (route.request().method() !== "POST") return route.fulfill({ json: [] });
    posts.push(route.request().postData() ?? "");
    return route.fulfill({ status: 201, json: image(7) });
  });
  await page.route(`**/lore/${mira.id}/images/7`, route => route.fulfill({ contentType: "image/webp", body: Buffer.from("RIFFimage") }));
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await page.getByRole("region", { name: "Series lore", exact: true }).getByRole("button", { name: /Mira/ }).click();
  const dialog = page.getByRole("dialog", { name: "Edit lore" });
  await expect(dialog.getByText(/Estimated cost: /)).toBeVisible();
  await dialog.getByRole("button", { name: "Generate image", exact: true }).click();
  await expect(dialog.getByRole("img", { name: "Image of Mira" })).toHaveAttribute("src", new RegExp(`/lore/${mira.id}/images/7$`));
  expect(posts).toEqual(["{}"]);
});
