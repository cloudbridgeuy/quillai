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

test("place lore chooses map or render and Leave out hides Generate", async ({ page, request }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Place series", position: 0 } })).json();
  const base = `${api}/series/${series.id}`;
  const book = await (await request.post(`${base}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${base}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Harbor and Vault.\n" }] },
  } })).json();
  const harbor = await (await request.post(`${base}/lore`, { data: { kind: "place", name: "Harbor", visible_text: "Salt docks." } })).json();
  await request.post(`${base}/lore`, { data: { kind: "place", name: "Vault", visible_text: "Sealed.", level: "leave_out" } });
  const posts: string[] = [];
  await page.route(`**/lore/${harbor.id}/images`, route => {
    if (route.request().method() !== "POST") return route.fulfill({ json: [] });
    posts.push(route.request().postData() ?? "");
    return route.fulfill({ status: 201, json: { id: posts.length, loreEntryId: harbor.id, timelinePointId: null, purpose: "map",
      timeOfDay: null, prompt: "p", model: "img1", size: "1024x1024", quality: "medium", mime: "image/webp", cost: 0.011, createdAt: "2026-11-06 10:00:00" } });
  });
  await page.route(`**/lore/${harbor.id}/images/*`, route => route.fulfill({ contentType: "image/webp", body: Buffer.from("RIFFimage") }));
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  const lore = page.getByRole("region", { name: "Series lore", exact: true });
  await lore.getByRole("button", { name: /Harbor/ }).click();
  const dialog = page.getByRole("dialog", { name: "Edit lore" });
  await expect(dialog.getByLabel("Time of day")).toHaveCount(0);
  await dialog.getByLabel("Image type").selectOption("map");
  await dialog.getByRole("button", { name: "Generate image", exact: true }).click();
  await dialog.getByLabel("Image type").selectOption("render");
  await expect(dialog.getByLabel("Time of day")).toHaveValue("day");
  await dialog.getByLabel("Time of day").selectOption("night");
  await dialog.getByRole("button", { name: "Generate image", exact: true }).click();
  await expect.poll(() => posts).toEqual([`{"purpose":"map"}`, `{"purpose":"render","timeOfDay":"night"}`]);
  await page.keyboard.press("Escape");
  await lore.getByRole("button", { name: /Vault/ }).click();
  await expect(page.getByRole("dialog", { name: "Edit lore" }).getByRole("button", { name: "Generate image" })).toHaveCount(0);
});
