import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import type Quill from "../app/node_modules/quill";
import { expectEditorReady } from "./editor-ready";

type Stream = { url: string; pieceIds: number[]; ready: number; changed: number; closed: number };

async function seed(request: APIRequestContext, api: string) {
  const post = async (url: string, data: object, status = 201) => {
    const response = await request.post(url, { data });
    expect(response.status(), await response.text()).toBe(status);
    return response.json();
  };
  const universe = await post(`${api}/universes`, { title: "Push script universe", position: 0 });
  const group = await post(`${api}/universes/${universe.id}/pieces`, { title: "Push script", position: 0, label: "screenplay", shape: "group" });
  const one = await post(`${api}/pieces/${group.id}/children`, { title: "First scene", position: 0, label: "scene" });
  const two = await post(`${api}/pieces/${group.id}/children`, { title: "Second scene", position: 1, label: "scene" });
  await post(`${api}/pieces/${one.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "First line.\n".repeat(100) }] } }, 200);
  await post(`${api}/pieces/${two.id}/deltas`, { baseRev: 0, delta: { ops: [{ insert: "Second scene.\n" }] } }, 200);
  return { group: group.id as number, one: one.id as number, two: two.id as number };
}

async function open(page: Page, api: string, target: Awaited<ReturnType<typeof seed>>, width: number, frozen: boolean) {
  await page.addInitScript(() => {
    const NativeEventSource = window.EventSource;
    const streams: Stream[] = [];
    class ObservedEventSource extends NativeEventSource {
      private record: Stream;
      constructor(url: string | URL, options?: EventSourceInit) {
        super(url, options);
        this.record = { url: this.url, pieceIds: [], ready: 0, changed: 0, closed: 0 };
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
  if (frozen) {
    await page.clock.install({ time: new Date("2026-01-01T00:00:00Z") });
    await page.clock.pauseAt(new Date("2026-01-02T00:00:00Z"));
  }
  const initialRead = page.waitForResponse(response => response.request().method() === "GET" && response.url().startsWith(`${api}/pieces/${target.group}/scenes/deltas?`) && response.status() === 200);
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, pieceId: String(target.group) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page.locator(".ql-editor")).toContainText("Second scene.");
  await expect.poll(async () => (await streams(page)).filter(source => source.ready > 0)).toHaveLength(1);
  await (await initialRead).finished();
  const observed = await streams(page);
  expect(observed).toHaveLength(1);
  expect(observed[0]!.closed).toBe(0);
  expect(observed[0]!.url).toBe(`${api}/sync/events?pieceIds=${target.one},${target.two}`);
  expect(observed[0]!.pieceIds).toEqual([target.one, target.two]);
}

async function streams(page: Page): Promise<Stream[]> {
  return page.evaluate(() => (window as unknown as { observedStreams: Stream[] }).observedStreams);
}

for (const width of [1100, 1440]) {
  test(`idle script has no delta reads at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const target = await seed(request, api);
    let reads = 0;
    page.on("request", sent => {
      if (sent.method() === "GET" && sent.url().startsWith(`${api}/pieces/${target.group}/scenes/deltas?`)) reads++;
    });
    await open(page, api, target, width, false);
    reads = 0;
    await page.waitForTimeout(5000);
    expect(reads).toBe(0);
    expect(await streams(page)).toHaveLength(1);
  });

  test(`remote non-active scene arrives through native stream and preserves scroll at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const target = await seed(request, api);
    await open(page, api, target, width, true);
    await page.locator(".ql-editor").evaluate(element => {
      type Fiber = { ref?: { current?: Quill }; child?: Fiber; sibling?: Fiber };
      for (let ancestor: Element | null = element; ancestor; ancestor = ancestor.parentElement) {
        const key = Object.keys(ancestor).find(name => name.startsWith("__reactFiber$"));
        const stack = key ? [(ancestor as unknown as Record<string, Fiber>)[key]!] : [];
        for (let visited = 0; stack.length && visited < 500; visited++) {
          const fiber = stack.pop()!;
          const quill = fiber.ref?.current;
          if (quill?.root === element) {
            quill.focus();
            quill.setSelection(1, 0, "user");
            if (quill.getSelection()?.index !== 1) throw new Error("First scene selection missing");
            return;
          }
          if (fiber.child) stack.push(fiber.child);
          if (fiber.sibling) stack.push(fiber.sibling);
        }
      }
      throw new Error("Mounted Quill instance not found");
    });
    expect(await page.locator(".ql-editor").evaluate(element => getComputedStyle(element).overflowY)).toBe("visible");
    const scroll = await page.evaluate(() => {
      const root = document.scrollingElement!;
      root.scrollTop = root.scrollHeight;
      return root.scrollTop;
    });
    expect(scroll).toBeGreaterThan(0);
    const before = (await streams(page))[0]!.changed;
    const start = Date.now();
    const response = await request.post(`${api}/pieces/${target.two}/deltas`, { data: { baseRev: 1, delta: { ops: [{ insert: "Remote " }] } } });
    expect(response.status(), await response.text()).toBe(200);
    await expect(page.locator(".ql-editor")).toContainText("Remote Second scene.", { timeout: Math.max(1, 1000 - (Date.now() - start)) });
    expect(Date.now() - start).toBeLessThanOrEqual(1000);
    expect(await page.evaluate(() => document.scrollingElement!.scrollTop)).toBe(scroll);
    const observed = await streams(page);
    expect(observed).toHaveLength(1);
    expect(observed[0]!.changed).toBeGreaterThan(before);
  });

  test(`deleted scene enters blocked state after fallback at ${width}px`, async ({ page, request }) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const target = await seed(request, api);
    await open(page, api, target, width, true);
    const before = (await streams(page))[0]!.changed;
    expect((await request.delete(`${api}/pieces/${target.two}`)).status()).toBe(204);
    const failure = page.waitForResponse(response => response.request().method() === "GET" && response.url().startsWith(`${api}/pieces/${target.group}/scenes/deltas?`) && response.status() === 422);
    await page.clock.runFor(16000);
    await failure;
    await expect(page.getByRole("alert", { name: "Chapter save status", exact: true })).toContainText("Save stopped");
    await expect(page.locator(".ql-editor")).toContainText("Second scene.");
    expect((await streams(page))[0]!.changed).toBe(before);
  });
}
