import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

type Sample = { opacity: number[]; window: boolean; loading: boolean };

async function startSampling(page: Page) {
  await page.evaluate(() => {
    const state = window as unknown as { flashSamples: Sample[]; sampling: boolean };
    state.flashSamples = [];
    state.sampling = true;
    const baseline = Array.from(document.querySelectorAll<HTMLButtonElement>(".editor-rail button, .ql-toolbar button, .editor-manuscript-tree button")).filter(element => !element.disabled && getComputedStyle(element).opacity === "1");
    const frame = () => {
      const controls = baseline.filter(element => element.isConnected);
      state.flashSamples.push({
        opacity: Array.from(controls, element => Number(getComputedStyle(element).opacity)),
        window: Boolean(document.querySelector(".editor-continue-window")),
        loading: document.querySelector('[aria-label="Chapter save status"]')?.textContent?.includes("Loading chapter…") ?? false,
      });
      if (state.sampling) requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
}

for (const width of [1100, 1440]) {
  for (const action of ["switch", "create"] as const) {
    for (const delay of [200, 1000]) {
      test(`${action} chapter with ${delay}ms delay at ${width}px`, async ({ page, request }) => {
        const api = process.env.QUILLAI_E2E_API_URL;
        if (!api) throw new Error("Run with cargo xtask e2e");
        const post = async (url: string, data: object) => {
          const response = await request.post(url, { data });
          expect(response.status()).toBe(201);
          return response.json();
        };
        const series = await post(`${api}/series`, { title: `Flash series ${width} ${action}`, position: 0 });
        const book = await post(`${api}/series/${series.id}/books`, { title: "Flash book", position: 0 });
        const chaptersUrl = `${api}/series/${series.id}/books/${book.id}/chapters`;
        const first = await post(chaptersUrl, { title: "Flash first", position: 0, kind: "chapter", content: { ops: [{ insert: "First chapter\n" }] } });
        await post(chaptersUrl, { title: "Flash second", position: 1, kind: "chapter", content: { ops: [{ insert: "Second chapter\n" }] } });
        await page.setViewportSize({ width, height: 900 });
        await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(first.id) })}`);
        const editor = page.locator(".ql-editor");
        await expectEditorReady(editor);
        await expect(page.locator(".editor-continue-window")).toHaveCount(1);
        await page.getByRole("button", { name: "Manuscript", exact: true }).click();
        const outline = page.getByRole("region", { name: "Manuscript outline" });
        await expect(outline.getByRole("button", { name: "Flash second", exact: true })).toBeVisible();
        if (action === "create") {
          await outline.getByRole("button", { name: "+ Chapter", exact: true }).click();
          await outline.getByRole("textbox", { name: "Chapter title" }).fill("Flash created");
        }
        let intercepted = 0;
        await page.route(`${chaptersUrl}/*`, async route => {
          if (route.request().method() === "GET" && /\/\d+$/.test(new URL(route.request().url()).pathname)) {
            intercepted++;
            await new Promise(resolve => setTimeout(resolve, delay));
          }
          await route.continue();
        });
        if (delay === 200) await startSampling(page);
        if (action === "switch") await outline.getByRole("button", { name: "Flash second", exact: true }).click();
        else await outline.getByRole("textbox", { name: "Chapter title" }).press("Enter");
        if (delay === 1000) {
          await expect(page.getByLabel("Chapter save status")).toHaveText("Loading chapter…");
          const continued = page.locator('.editor-rail button[aria-label="Continue writing"]');
          await expect(continued).toBeDisabled();
          await expect(continued).toHaveCSS("opacity", "0.5");
        }
        await expect(page.locator(".editor-chapter-title")).toHaveText(action === "switch" ? "Flash second" : "Flash created");
        await expectEditorReady(editor);
        if (delay === 200) {
          const samples = await page.evaluate(() => {
            const state = window as unknown as { flashSamples: Sample[]; sampling: boolean };
            state.sampling = false;
            return state.flashSamples;
          });
          expect(samples.length).toBeGreaterThan(2);
          expect(samples.every(sample => sample.opacity.length >= 5 && sample.opacity.every(value => value === 1) && sample.window && !sample.loading)).toBe(true);
        }
        expect(intercepted).toBeGreaterThan(0);
      });
    }
  }
}
