import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";

const rails = [
  { name: "Library", title: "Back to library", glyph: "⌂" },
  { name: "Manuscript", title: "Manuscript (Ctrl/Cmd+[)", glyph: "☰" },
  { name: "Continue writing", title: "Continue writing", glyph: "➤" },
  { name: "Edit selection", title: "Edit selection", glyph: "✎" },
  { name: "Context inspector", title: "Context inspector (Ctrl/Cmd+])", glyph: "◫" },
  { name: "Settings", title: "Settings", glyph: "⚙" },
];

async function setup(page: Page, request: APIRequestContext) {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identity = `GUZ868-1-${crypto.randomUUID()}`;
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Rail series", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Rail book", position: 0 });
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Rail chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `Opening line. ${identity}\n` }] },
  });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  return { identity, provider };
}

async function selectText(page: Page) {
  const editor = page.locator(".ql-editor");
  await editor.click();
  await editor.evaluate((element) => {
    const text = element.querySelector("p")?.firstChild;
    if (!text) throw new Error("Missing chapter text");
    const range = document.createRange();
    range.setStart(text, 0);
    range.setEnd(text, 7);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  });
  await expect(page.getByRole("button", { name: "Edit selection", exact: true })).toBeEnabled();
}

test("rail glyph centres, names, and selection at desktop widths", async ({ page, request }, testInfo) => {
  await setup(page, request);
  const measurements: { width: number; name: string; x: number; y: number }[] = [];
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    await selectText(page);
    for (const rail of rails) {
      const button = page.getByRole("button", { name: rail.name, exact: true });
      await expect(button).toHaveAccessibleName(rail.name);
      await expect(button).toHaveAttribute("title", rail.title);
      const delta = await button.evaluate((element, glyph) => {
        const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
        let node: Node | null;
        while ((node = walker.nextNode())) {
          const index = node.textContent?.indexOf(glyph) ?? -1;
          if (index < 0) continue;
          const range = document.createRange();
          range.setStart(node, index);
          range.setEnd(node, index + glyph.length);
          const text = range.getBoundingClientRect();
          let top = text.top;
          let bottom = text.bottom;
          if (glyph === "➤") {
            let label: Node | null;
            while ((label = walker.nextNode()) && label.textContent?.trim() !== "Continue") {}
            if (!label) throw new Error("Missing Continue label");
            const labelRange = document.createRange();
            labelRange.selectNodeContents(label);
            const labelRect = labelRange.getBoundingClientRect();
            top = Math.min(top, labelRect.top);
            bottom = Math.max(bottom, labelRect.bottom);
          }
          const button = element.getBoundingClientRect();
          return { x: text.x + text.width / 2 - button.x - button.width / 2, y: (top + bottom) / 2 - button.y - button.height / 2 };
        }
        throw new Error(`Missing glyph ${glyph}`);
      }, rail.glyph);
      measurements.push({ width, name: rail.name, ...delta });
    }
  }
  console.log(JSON.stringify(measurements));
  await testInfo.attach("rail-glyph-centres.json", { body: JSON.stringify(measurements, null, 2), contentType: "application/json" });
  for (const measurement of measurements) {
    expect.soft(Math.abs(measurement.x), `${measurement.width} ${measurement.name} x`).toBeLessThanOrEqual(1);
    expect.soft(Math.abs(measurement.y), `${measurement.width} ${measurement.name} y`).toBeLessThanOrEqual(measurement.name === "Continue writing" ? 1.5 : 1);
  }
});

test("rail mousedown preserves selection and panels toggle", async ({ page, request }) => {
  await setup(page, request);
  for (const width of [1100, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const rail of rails) {
      await selectText(page);
      const button = page.getByRole("button", { name: rail.name, exact: true });
      await expect(button).toBeEnabled();
      await page.evaluate(() => {
        document.addEventListener("mousedown", (event) => {
          document.documentElement.dataset.railPrevented = String(event.defaultPrevented);
        }, { once: true });
      });
      const box = await button.boundingBox();
      if (!box) throw new Error(`Missing button ${rail.name}`);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(0, 500);
      await page.mouse.up();
      expect(await page.evaluate(() => document.documentElement.dataset.railPrevented)).toBe("true");
      expect(await page.evaluate(() => {
        const selection = window.getSelection();
        const text = document.querySelector(".ql-editor p")?.firstChild;
        if (!selection || !text) throw new Error("Missing editor selection");
        return {
          anchor: selection.anchorNode === text,
          anchorOffset: selection.anchorOffset,
          focus: selection.focusNode === text,
          focusOffset: selection.focusOffset,
          ranges: Array.from({ length: selection.rangeCount }, (_, index) => {
            const range = selection.getRangeAt(index);
            return { start: range.startContainer === text, startOffset: range.startOffset, end: range.endContainer === text, endOffset: range.endOffset };
          }),
        };
      })).toEqual({ anchor: true, anchorOffset: 0, focus: true, focusOffset: 7, ranges: [{ start: true, startOffset: 0, end: true, endOffset: 7 }] });
    }
    for (const [name, selector] of [["Manuscript", "#manuscript-drawer"], ["Context inspector", "#context-drawer"], ["Edit selection", "#edit-instruction-box"], ["Settings", '[role="dialog"][aria-labelledby="editor-settings-title"]']]) {
      await selectText(page);
      const button = page.getByRole("button", { name, exact: true });
      const panel = page.locator(selector);
      await button.click();
      await expect(panel).toBeVisible();
      await button.click();
      await expect(panel).toBeHidden();
    }
  }
});

for (const width of [1100, 1440]) {
  test(`Continue exposes the pending thread indicator at ${width}px`, async ({ page, request }) => {
    await page.setViewportSize({ width, height: 1000 });
    const { identity, provider } = await setup(page, request);
    const context = page.getByRole("button", { name: "Context inspector", exact: true });
    await context.click();
    await page.getByText("Advanced AI controls", { exact: true }).click();
    await page.getByRole("combobox", { name: "Continue model", exact: true }).selectOption("e2e-continue");
    const before = await (await request.get(`${provider}/control`)).json();
    expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
    await page.getByRole("button", { name: "Continue writing", exact: true }).click();
    const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
    expect(upstream.identity).toBe(identity);
    const stop = page.getByRole("button", { name: "Stop AI", exact: true });
    await expect(stop).toHaveAccessibleName("Stop AI");
    await expect(stop).toHaveAttribute("title", "Stop AI");
    const control = `${provider}/control/${upstream.id}`;
    expect((await request.post(`${control}/fragment`, { data: { text: "A cut-off reply." } })).status()).toBe(200);
    expect((await request.post(`${control}/finish`, { data: { reason: "length" } })).status()).toBe(200);
    const resume = page.getByRole("button", { name: "Continue writing from a cut-off reply", exact: true });
    await expect(resume).toHaveAttribute("title", "Continue writing from a cut-off reply");
    await expect(resume.locator(".editor-thread-indicator")).toHaveText("from a cut-off reply");
  });
}
