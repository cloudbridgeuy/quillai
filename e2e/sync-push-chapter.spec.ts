import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

type Scope = "chapter" | "piece";
type Stream = { url: string; ready: number; pieceIds: number[]; changed: number; closed: number };

async function seed(request: APIRequestContext, api: string, scope: Scope) {
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  if (scope === "piece") {
    const universe = await post(`${api}/universes`, { title: "Push universe", position: 0 });
    const piece = await post(`${api}/universes/${universe.id}/pieces`, { title: "Push piece", position: 0, label: "short_story", shape: "leaf" });
    const url = `${api}/pieces/${piece.id}`;
    await post(`${url}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Opening " }] } }, 200);
    return { url, streamUrl: `${api}/sync/events?pieceIds=${piece.id}`, rev: 1, query: new URLSearchParams({ apiBaseUrl: api, pieceId: String(piece.id) }) };
  }
  const series = await post(`${api}/series`, { title: "Push series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Push book", position: 0 });
  const base = `${api}/series/${series.id}/books/${book.id}/chapters`;
  const chapter = await post(base, { title: "Push chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening \n" }] } });
  return { url: `${base}/${chapter.id}`, streamUrl: `${base}/${chapter.id}/events`, rev: Number(chapter.head_rev), query: new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) }) };
}

type Target = Awaited<ReturnType<typeof seed>>;

async function observeStreams(page: Page) {
  await page.addInitScript(() => {
    const NativeEventSource = window.EventSource;
    const streams: { url: string; ready: number; pieceIds: number[]; changed: number; closed: number }[] = [];
    class ObservedEventSource extends NativeEventSource {
      private record: (typeof streams)[number];
      constructor(url: string | URL, options?: EventSourceInit) {
        super(url, options);
        this.record = { url: this.url, ready: 0, pieceIds: [], changed: 0, closed: 0 };
        streams.push(this.record);
        this.addEventListener("ready", event => {
          this.record.pieceIds = JSON.parse(event.data).pieceIds;
          this.record.ready++;
        });
        this.addEventListener("changed", () => this.record.changed++);
      }
      close() {
        this.record.closed++;
        super.close();
      }
    }
    Object.assign(window, { EventSource: ObservedEventSource, observedStreams: streams });
  });
}

async function streams(page: Page): Promise<Stream[]> {
  return page.evaluate(() => (window as unknown as { observedStreams: Stream[] }).observedStreams);
}

async function open(page: Page, target: Target, width: number, frozen = false) {
  await observeStreams(page);
  if (frozen) {
    await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
    await page.clock.pauseAt(new Date("2026-01-02T00:00:00Z"));
  }
  const initialRead = page.waitForResponse(response => response.request().method() === "GET" && response.url().startsWith(`${target.url}/deltas?`) && response.status() === 200);
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${target.query}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page.locator(".ql-editor")).toContainText("Opening ");
  const settledRead = initialRead.then(response => response.finished());
  if (frozen) await Promise.all([ready(page, target), settledRead]);
  else await settledRead;
  await page.evaluate(() => new Promise<void>(resolve => queueMicrotask(resolve)));
}

async function ready(page: Page, target: Target) {
  await expect.poll(async () => (await streams(page)).filter(source => source.url === target.streamUrl && source.ready > 0)).toHaveLength(1);
  const observed = await streams(page);
  expect(observed).toHaveLength(1);
  const source = observed[0]!;
  expect(source.closed).toBe(0);
  expect(source.pieceIds).toHaveLength(1);
  expect(Number.isSafeInteger(source.pieceIds[0]) && source.pieceIds[0]! > 0).toBe(true);
  return source;
}

async function remoteSave(request: APIRequestContext, url: string, rev: number, text: string) {
  const response = await request.post(`${url}/deltas`, { data: { baseRev: rev, delta: { ops: [{ insert: text }] } } });
  const completed = Date.now();
  expect(response.status(), await response.text()).toBe(200);
  return completed;
}

for (const width of [1100, 1440]) {
  for (const scope of ["chapter", "piece"] as const) {
    test(`idle ${scope} has no delta reads at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const target = await seed(request, api, scope);
      let reads = 0;
      page.on("request", request => {
        if (request.method() === "GET" && request.url().startsWith(`${target.url}/deltas?`)) reads++;
      });
      await open(page, target, width);
      reads = 0;
      await page.waitForTimeout(5000);
      expect(reads).toBe(0);
    });

    test(`remote ${scope} save arrives through native stream at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const target = await seed(request, api, scope);
      await open(page, target, width, true);
      const before = await ready(page, target);
      const start = await remoteSave(request, target.url, target.rev, "Remote ");
      await expect(page.locator(".ql-editor")).toContainText("Remote Opening", { timeout: Math.max(1, 1500 - (Date.now() - start)) });
      expect(Date.now() - start).toBeLessThanOrEqual(1500);
      expect((await streams(page)).find(source => source.url === target.streamUrl)!.changed).toBeGreaterThan(before.changed);
    });

    test(`deleted open ${scope} enters blocked state after fallback at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const target = await seed(request, api, scope);
      await open(page, target, width, true);
      const active = await ready(page, target);
      const before = await streams(page);
      expect((await request.delete(`${api}/pieces/${active.pieceIds[0]}`)).status()).toBe(204);
      const failure = page.waitForResponse(response => response.request().method() === "GET" && response.url().startsWith(`${target.url}/deltas?`) && response.status() === 404);
      await page.clock.runFor(16000);
      await failure;
      await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toContainText("Save stopped");
      await expect(page.locator(".ql-editor")).toContainText("Opening ");
      expect((await streams(page)).reduce((sum, source) => sum + source.changed, 0)).toBe(before.reduce((sum, source) => sum + source.changed, 0));
    });

    test(`${scope} pagehide closes and pageshow refetches at ${width}px`, async ({ page, request }) => {
      const api = process.env.QUILLAI_E2E_API_URL;
      if (!api) throw new Error("Run with cargo xtask e2e");
      const target = await seed(request, api, scope);
      await open(page, target, width, true);
      const before = await streams(page);
      await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true })));
      await expect.poll(async () => (await streams(page)).some(source => source.closed > 0)).toBe(true);
      await remoteSave(request, target.url, target.rev, "Hidden ");
      await new Promise(resolve => setTimeout(resolve, 200));
      await expect(page.locator(".ql-editor")).not.toContainText("Hidden ");
      const refetch = page.waitForResponse(response => response.request().method() === "GET" && response.url().startsWith(`${target.url}/deltas?`) && response.status() === 200);
      await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })));
      await expect.poll(async () => (await streams(page)).length).toBeGreaterThan(before.length);
      await expect.poll(async () => (await streams(page)).slice(before.length).some(source => source.ready > 0)).toBe(true);
      await refetch;
      await expect(page.locator(".ql-editor")).toContainText("Hidden Opening");
    });
  }
}
