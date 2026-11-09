import { expect, test, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

const BUSY_DELAY_MS = 300;

type Sample = { at: number; opacity: number[]; window: boolean; loading: boolean; settled: boolean; continueDisabled: boolean; continueOpacity: number };
type SamplerState = { flashSamples: Sample[]; sampling: boolean; flashStart: number | null };

async function startSampling(page: Page, actionEvent: "click" | "keydown", expectedTitle: string) {
  await page.evaluate(({ actionEvent, expectedTitle }) => {
    const state = window as unknown as SamplerState;
    state.flashSamples = [];
    state.sampling = true;
    state.flashStart = null;
    window.addEventListener(actionEvent, () => { state.flashStart = performance.now(); }, { capture: true, once: true });
    const baseline = Array.from(document.querySelectorAll<HTMLButtonElement>(".editor-rail button, .ql-toolbar button, .editor-manuscript-tree button")).filter(element => !element.disabled && getComputedStyle(element).opacity === "1");
    const frame = () => {
      const controls = baseline.filter(element => element.isConnected);
      const continueButton = document.querySelector<HTMLButtonElement>('.editor-rail button[aria-label="Continue writing"]');
      state.flashSamples.push({
        at: performance.now(),
        opacity: Array.from(controls, element => Number(getComputedStyle(element).opacity)),
        window: Boolean(document.querySelector(".editor-continue-window")),
        loading: document.querySelector('[aria-label="Chapter save status"]')?.textContent?.includes("Loading chapter…") ?? false,
        settled: document.querySelector(".editor-chapter-title")?.textContent === expectedTitle && document.querySelector(".ql-editor")?.getAttribute("contenteditable") === "true",
        continueDisabled: continueButton?.disabled ?? false,
        continueOpacity: continueButton ? Number(getComputedStyle(continueButton).opacity) : Number.NaN,
      });
      if (state.sampling) requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  }, { actionEvent, expectedTitle });
}

async function stopSampling(page: Page) {
  return page.evaluate(async () => {
    const state = window as unknown as SamplerState;
    for (let frames = 0; frames < 2; frames++) await new Promise(requestAnimationFrame);
    state.sampling = false;
    return { samples: state.flashSamples, start: state.flashStart };
  });
}

function flashClause(sample: Sample): string | null {
  if (sample.opacity.length < 5) return "fewer than 5 controls";
  if (!sample.opacity.every(value => value === 1)) return "opacity not 1";
  if (!sample.window) return "continue window missing";
  if (sample.loading) return "loading status shown";
  return null;
}

function flashFaults(samples: Sample[], start: number) {
  return samples.flatMap(sample => {
    const clause = flashClause(sample);
    return clause ? [{ clause, t: Math.round(sample.at - start), sample }] : [];
  });
}

for (const width of [1100, 1440]) {
  for (const action of ["switch", "create"] as const) {
    for (const delay of action === "switch" ? [50, 1000] : [200, 1000]) {
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
        const expectedTitle = action === "switch" ? "Flash second" : "Flash created";
        await startSampling(page, action === "switch" ? "click" : "keydown", expectedTitle);
        if (action === "switch") await outline.getByRole("button", { name: "Flash second", exact: true }).click();
        else await outline.getByRole("textbox", { name: "Chapter title" }).press("Enter");
        await expect(page.locator(".editor-chapter-title")).toHaveText(expectedTitle);
        await expectEditorReady(editor);
        const { samples, start } = await stopSampling(page);
        expect(samples.length).toBeGreaterThan(2);
        if (delay === 1000) {
          const states = samples.map(({ loading, continueDisabled, continueOpacity }) => ({ loading, continueDisabled, continueOpacity }));
          expect(states.some(state => state.loading && state.continueDisabled && state.continueOpacity === 0.5), `no single busy sample: ${JSON.stringify(states.slice(-20))}`).toBe(true);
        } else {
          expect(start, "action event never reached the page").not.toBeNull();
          const actionStart = start as number;
          const beforeBusy = samples.filter(sample => sample.at < actionStart + BUSY_DELAY_MS);
          const afterLoad = samples.slice(samples.findIndex(sample => sample.at >= actionStart && sample.settled));
          expect(beforeBusy.filter(sample => sample.at >= actionStart).length, "no sample in the first 300ms after the action").toBeGreaterThan(0);
          expect(samples.some(sample => sample.at >= actionStart && sample.settled), "no sample after load finished").toBe(true);
          expect(flashFaults(beforeBusy, actionStart), "flash before the busy threshold").toEqual([]);
          expect(flashFaults(afterLoad, actionStart), "flash after load finished").toEqual([]);
        }
        expect(intercepted).toBeGreaterThan(0);
      });
    }
  }
}
