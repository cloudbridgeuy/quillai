import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";
import { expectSaved } from "./save-state";

for (const available of [true, false]) {
  test(`requests carry no model and the Inspector shows the Settings model ${available ? "from the catalog" : "or unknown when the catalog is unavailable"}`, async ({ page, request }) => {
    const apiUrl = process.env.QUILLAI_E2E_API_URL;
    if (!apiUrl) throw new Error("Run with cargo xtask e2e");
    const models = [
      { id: "provider/fast", label: "Fast", input_price_per_million: 0.5, output_price_per_million: 1 },
      { id: "provider/precise", label: "Precise", input_price_per_million: 2, output_price_per_million: 6 },
    ];
    await page.route(`${apiUrl}/ai/models`, (route) => route.fulfill(available ? {
      json: { models, defaults: { generate: models[1]!.id, edit: models[0]!.id } },
    } : { status: 500, json: { error: "Unavailable" } }));
    // Settings holds the Continue and Edit models; the catalog defaults mirror them.
    await page.route(`${apiUrl}/settings/ai`, (route) => route.fulfill({ json: {
      base_url: "https://env.example/v1", generate_model: models[1]!.id, edit_model: models[0]!.id, key_set: true,
      sources: { base_url: "env", api_key: "env", generate_model: "saved", edit_model: "saved" } } }));
    const bodies: Array<{ action: string; model?: string }> = [];
    await page.route(`${apiUrl}/ai/generate`, async (route) => {
      bodies.push(route.request().postDataJSON());
      await route.fulfill({ contentType: "application/x-ndjson", body:
        `${JSON.stringify({ type: "delta", delta: { ops: [{ insert: "Revised" }] } })}\n${JSON.stringify({ type: "usage", promptTokens: 1000, completionTokens: 500 })}\n${JSON.stringify({ type: "done", finishReason: "stop" })}\n` });
    });
    const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Models series", position: 0 } });
    expect(seriesResponse.status()).toBe(201);
    const series = await seriesResponse.json();
    const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Models book", position: 0 } });
    expect(bookResponse.status()).toBe(201);
    const book = await bookResponse.json();
    const chapterResponse = await request.post(`${apiUrl}/series/${series.id}/books/${book.id}/chapters`, {
      data: { title: "Models chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "Opening line\n" }] } },
    });
    expect(chapterResponse.status()).toBe(201);
    const chapter = await chapterResponse.json();
    await page.setViewportSize({ width: 1100, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
    const editor = page.locator(".ql-editor");
    const drawer = page.locator("#context-drawer");
    const scrollArea = drawer.locator(".editor-context-body");
    const contextButton = page.getByRole("button", { name: "Context inspector", exact: true });
    const modelLine = (region: ReturnType<typeof drawer.getByRole>) => region.locator("dt").filter({ hasText: /^Model$/ }).locator("+ dd");
    const maxOutput = drawer.getByRole("spinbutton", { name: "Max Output Tokens", exact: true });
    async function openAdvanced() {
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      await drawer.getByText("Advanced AI controls", { exact: true }).click();
    }
    await expectSaved(page);
    await expectEditorReady(editor);
    await openAdvanced();
    // The per-chapter model menus are gone; Settings is the one place to choose.
    await expect(drawer.getByRole("combobox", { name: "Continue model", exact: true })).toHaveCount(0);
    await expect(drawer.getByRole("combobox", { name: "Edit model", exact: true })).toHaveCount(0);
    const region = drawer.getByRole("region", { name: "Model and cost", exact: true });
    await region.scrollIntoViewIfNeeded();
    if (available) {
      await expect(modelLine(region)).toHaveText("Precise");
      await expect(modelLine(region)).toHaveAttribute("title", models[1]!.id);
    } else {
      await expect(modelLine(region)).toHaveText("unknown — unresolved model");
    }
    await page.reload();
    await expectSaved(page);
    await openAdvanced();
    for (const width of [1100, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      await expect(page.getByRole("button", { name: "Close context", exact: true })).toHaveCount(0);
      await expect(contextButton).toHaveAttribute("aria-expanded", "true");
      await editor.focus();
      await page.mouse.move(500, 300);
      await expect(page.locator(".editor-rail-right")).toHaveCSS("opacity", "0.95");
      await expect(contextButton).toHaveCSS("color", "rgb(31, 41, 51)");
      await expect(contextButton).toHaveCSS("background-color", "rgb(215, 230, 208)");
      await contextButton.click();
      await expect(drawer).toBeHidden();
      await expect(contextButton).toHaveAttribute("aria-expanded", "false");
      await expect(editor).toBeFocused();
      await page.mouse.move(500, 300);
      await expect(page.locator(".editor-rail-right")).toHaveCSS("opacity", "0.35");
      await contextButton.click();
      await expect(drawer).toBeVisible();
      await expect(contextButton).toHaveAttribute("aria-expanded", "true");
      await page.keyboard.press("Escape");
      await expect(drawer).toBeHidden();
      await expect(contextButton).toHaveAttribute("aria-expanded", "false");
      await expect(editor).toBeFocused();
      await page.keyboard.press(width === 1100 ? "Control+]" : "Meta+]");
      await expect(drawer).toBeVisible();
      await expect(contextButton).toHaveAttribute("aria-expanded", "true");
      await page.mouse.move(500, 300);
      await expect(page.locator(".editor-rail-right")).toHaveCSS("opacity", "0.95");
      await expect(contextButton).toHaveCSS("color", "rgb(31, 41, 51)");
      await expect(contextButton).toHaveCSS("background-color", "rgb(215, 230, 208)");
      await expect(drawer).toHaveCSS("overflow-y", "hidden");
      await expect(drawer).toHaveCSS("overscroll-behavior-y", "contain");
      await expect(scrollArea).toHaveCSS("overflow-y", "auto");
      await expect(scrollArea).toHaveCSS("overscroll-behavior-y", "contain");
      for (const name of ["Story Context", "Continue instruction", "Thread continuation prompt", "Author guidance"]) {
        const note = drawer.getByRole("textbox", { name, exact: true });
        await expect(note).toHaveClass(/editor-sticky-note/);
        await expect(note).toHaveCSS("background-color", "rgb(244, 232, 181)");
        await expect(note).toHaveCSS("border-top-width", "0px");
        await expect(note).toHaveCSS("font-size", "13px");
        expect(await note.evaluate(node => getComputedStyle(node, "::placeholder").fontSize)).toBe("13px");
      }
      for (const name of ["Story Context", "Continue instruction", "Max Output Tokens"]) {
        await expect(drawer.locator("label > span").filter({ hasText: new RegExp(`^${name}$`) }))
          .toHaveCSS("font-size", "12px");
      }
      const sizes = await drawer.evaluate(element => [...new Set([...element.querySelectorAll("*")]
        .filter(node => node.getClientRects().length > 0 && (node.matches("input, select, textarea") ||
          [...node.childNodes].some(child => child.nodeType === Node.TEXT_NODE && child.textContent?.trim())))
        .map(node => getComputedStyle(node).fontSize))].sort());
      expect(sizes).toEqual(["12px", "13px"]);
      await expect(drawer.getByRole("textbox", { name: "System prompt section", exact: true })).toHaveAttribute("readonly", "");
      await expect(drawer.getByRole("textbox", { name: "System prompt section", exact: true })).not.toHaveClass(/editor-sticky-note/);
      await scrollArea.evaluate(node => { node.scrollTop = 0; });
      expect(await drawer.evaluate(node => ({ top: node.getBoundingClientRect().top, bottom: node.getBoundingClientRect().bottom })))
        .toEqual({ top: 0, bottom: 1000 });
      const header = drawer.locator(":scope > .editor-drawer-header");
      const heading = drawer.getByRole("heading", { name: "Live Prompt Context", exact: true });
      await expect(header).toHaveCSS("background-color", "rgb(255, 255, 255)");
      await expect(header).toHaveCSS("border-bottom-width", "1px");
      await expect(header).toHaveCSS("border-bottom-color", "rgb(237, 227, 211)");
      const headerOffsets = await header.evaluate(node => ({ top: node.getBoundingClientRect().top,
        height: node.getBoundingClientRect().height,
        headingTop: node.querySelector("h2")!.getBoundingClientRect().top }));
      const centers = await page.evaluate(() => ["#context-inspector-title", ".editor-topbar h1", ".editor-continue > span:first-child"]
        .map(selector => {
          const rect = document.querySelector(selector)!.getBoundingClientRect();
          return rect.top + rect.height / 2;
        }));
      expect(Math.max(...centers) - Math.min(...centers)).toBeLessThanOrEqual(2);
      expect(await scrollArea.evaluate(node => node.getBoundingClientRect().top))
        .toBe(headerOffsets.top + headerOffsets.height);
      console.log(JSON.stringify({ width, catalogAvailable: available, fixedHeader: headerOffsets, headingCenters: centers }));
      if (available) await page.screenshot({ path: `../target/guz-680/${test.info().project.name}/drawer-top-${width}.png` });
      if (available) await page.screenshot({ path: `../target/guz-680/${test.info().project.name}/rail-active-${width}.png` });
      await drawer.locator(".editor-advanced-section").scrollIntoViewIfNeeded();
      await expect(maxOutput).toBeInViewport();
      await page.screenshot({ path: `../target/guz-680/${test.info().project.name}/${available ? "models" : "unavailable"}-${width}.png` });
      const pageScroll = await page.evaluate(() => window.scrollY);
      await scrollArea.evaluate(node => { node.scrollTop = node.scrollHeight; });
      expect(await drawer.evaluate(node => ({ top: node.getBoundingClientRect().top, bottom: node.getBoundingClientRect().bottom })))
        .toEqual({ top: 0, bottom: 1000 });
      expect(await scrollArea.evaluate(node => Math.abs(node.scrollHeight - node.clientHeight - node.scrollTop) <= 1)).toBe(true);
      expect(await page.evaluate(() => window.scrollY)).toBe(pageScroll);
      await expect(heading).toBeInViewport({ ratio: 1 });
      expect(await header.evaluate(node => ({ top: node.getBoundingClientRect().top,
        height: node.getBoundingClientRect().height,
        headingTop: node.querySelector("h2")!.getBoundingClientRect().top }))).toEqual(headerOffsets);
      expect(await scrollArea.evaluate(node => node.getBoundingClientRect().top))
        .toBe(headerOffsets.top + headerOffsets.height);
      expect(await header.evaluate(node => {
        const rect = node.getBoundingClientRect();
        return [0.05, 0.5, 0.95].every(x => [0.05, 0.5, 0.95].every(y =>
          node.contains(document.elementFromPoint(rect.left + rect.width * x, rect.top + rect.height * y))));
      })).toBe(true);
      if (available) await page.screenshot({ path: `../target/guz-680/${test.info().project.name}/drawer-bottom-${width}.png` });
    }
    await contextButton.click();
    await editor.click();
    await page.locator(".editor-continue").click();
    await expect(editor).toContainText("Revised");
    await expectSaved(page);
    await contextButton.click();
    const cost = drawer.getByRole("region", { name: "Model and cost", exact: true });
    await cost.scrollIntoViewIfNeeded();
    await expect(cost.getByText("1000 input · 500 output tokens", { exact: true })).toBeVisible();
    await expect(cost.locator("dd").last()).toHaveText(available ? "$0.005" : "unknown — unresolved model");
    await contextButton.click();
    await editor.click();
    await editor.evaluate((element) => {
      const text = element.querySelector("p")?.firstChild;
      if (!text) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 3);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
    });
    const edit = page.getByRole("button", { name: "Edit selection", exact: true });
    await expect(edit).toBeEnabled();
    await edit.click();
    const note = page.getByRole("textbox", { name: "Edit instruction", exact: true });
    await note.fill("Rewrite");
    await note.press("Enter");
    await expect(note).toBeHidden();
    await expect.poll(() => bodies.length).toBe(2);
    await expect(page.locator(".editor-continue")).toHaveAccessibleName("Continue writing");
    await expectSaved(page);
    expect(bodies).toHaveLength(2);
    expect(bodies.map(({ action }) => action)).toEqual(["generate", "edit"]);
    for (const body of bodies) expect(body).not.toHaveProperty("model");
    await contextButton.click();
    await drawer.getByRole("combobox", { name: "Request to inspect", exact: true }).selectOption("edit");
    for (const width of [1100, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      await cost.scrollIntoViewIfNeeded();
      await expect(cost.getByRole("heading", { name: "Pending estimate", exact: true })).toBeVisible();
      await expect(cost.getByRole("heading", { name: "Most recent run · actual", exact: true })).toBeVisible();
      await expect(cost.getByText("1000 input · 500 output tokens", { exact: true })).toBeVisible();
      await expect(cost.locator("dd").last()).toHaveText(available ? "$0.001" : "unknown — unresolved model");
      await page.screenshot({ path: `../target/guz-681/${test.info().project.name}/${available ? "cost" : "unknown"}-${width}.png` });
    }
  });
}
