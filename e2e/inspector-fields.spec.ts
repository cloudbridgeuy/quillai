import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function openEditor(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const series = await (await request.post(`${api}/series`, { data: { title: "Inspector fields", position: 0 } })).json();
  const book = await (await request.post(`${api}/series/${series.id}/books`, { data: { title: "Book", position: 0 } })).json();
  const chapter = await (await request.post(`${api}/series/${series.id}/books/${book.id}/chapters`, { data: {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "A quiet harbour.\n" }] },
  } })).json();
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
}

async function expectUnbrokenWords(headings: Locator, minimum: number) {
  expect(await headings.count()).toBeGreaterThanOrEqual(minimum);
  for (const heading of await headings.all()) {
    await expect(heading).toBeVisible();
    const broken = await heading.evaluate(element => {
      const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
      const words: string[] = [];
      while (walker.nextNode()) {
        const node = walker.currentNode;
        const text = node.textContent ?? "";
        for (const match of text.matchAll(/\S+/g)) {
          const range = document.createRange();
          range.setStart(node, match.index!);
          range.setEnd(node, match.index! + match[0].length);
          const tops = Array.from(range.getClientRects(), rect => Math.round(rect.top));
          if (new Set(tops).size > 1) words.push(match[0]);
        }
      }
      return words;
    });
    expect(broken, `broken words in ${await heading.innerText()}`).toEqual([]);
  }
}

async function expectField(scope: Locator, name: string, role: "textbox" | "combobox" | "spinbutton", tag: string, type?: string) {
  const control = scope.getByRole(role, { name, exact: true });
  const visibleLabel = name.endsWith(" prompt section") ? name.slice(0, -" prompt section".length) : name;
  await expect(scope.getByText(visibleLabel, { exact: true })).toBeVisible();
  await expect(control).toBeVisible();
  await expect(control).toHaveJSProperty("tagName", tag.toUpperCase());
  if (type) await expect(control).toHaveJSProperty("type", type);
  return control;
}

for (const width of [1100, 1440]) {
  test.describe(`${width}px`, () => {
    test.beforeEach(async ({ page, request }) => {
      await openEditor(page, request, width);
    });

    test("headings keep words unbroken", async ({ page }) => {
      await expectUnbrokenWords(page.getByRole("heading", { name: "QuillAI", exact: true }), 1);
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      const settings = page.getByRole("dialog", { name: "Settings", exact: true });
      await expect(settings).toBeVisible();
      await expectUnbrokenWords(settings.getByRole("heading"), 1);
      await page.keyboard.press("Escape");

      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      const inspector = page.locator("#context-drawer");
      await expect(inspector.getByRole("heading", { name: "System", exact: true })).toBeVisible();
      await expectUnbrokenWords(inspector.locator(':is(h1, h2, h3, h4, h5, h6):not([aria-label="Model and cost"] *)'), 2);
      await page.getByRole("button", { name: "Context inspector", exact: true }).click();

      await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      const manuscript = page.locator("#manuscript-drawer");
      await expect(manuscript.getByRole("heading", { name: "Manuscript", exact: true })).toBeVisible();
      await expect(manuscript.getByRole("heading", { name: "Series lore", exact: true })).toBeVisible();
      await expectUnbrokenWords(manuscript.getByRole("heading"), 2);
      await manuscript.getByRole("region", { name: "Series lore", exact: true }).getByRole("button", { name: "New lore", exact: true }).click();
      const lore = page.getByRole("dialog", { name: "Create lore", exact: true });
      await expect(lore).toBeVisible();
      await expectUnbrokenWords(lore.getByRole("heading"), 1);
    });

    test("topbar heading uses 16px text", async ({ page }) => {
      const topbar = page.getByRole("heading", { name: "QuillAI", exact: true });
      await expect(topbar).toBeVisible();
      await expect(topbar).toHaveCSS("font-size", "16px");
    });

    test("inspector and settings fields have accessible labels and semantic controls", async ({ page }) => {
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      const settings = page.getByRole("dialog", { name: "Settings", exact: true });
      await expect(settings).toBeVisible();
      await expectField(settings, "Window Size", "spinbutton", "input", "number");
      await page.keyboard.press("Escape");

      await page.getByRole("button", { name: "Context inspector", exact: true }).click();
      const inspector = page.locator("#context-drawer");
      await expect(inspector.getByRole("heading", { name: "Live Prompt Context", exact: true })).toBeVisible();
      await inspector.getByText("Advanced AI controls", { exact: true }).click();
      for (const name of ["Continue model", "Edit model"]) await expect(inspector.getByRole("combobox", { name, exact: true })).toHaveCount(0);
      await expectField(inspector, "Max Output Tokens", "spinbutton", "input", "number");
      await expectField(inspector, "Request to inspect", "combobox", "select");
      await expectField(inspector, "Author guidance", "textbox", "textarea");
      const system = await expectField(inspector, "System prompt section", "textbox", "textarea");
      await expect(system).toHaveAttribute("readonly", "");
      const document = await expectField(inspector, "Document prompt section", "textbox", "textarea");
      await expect(document).toHaveAttribute("readonly", "");
      for (const role of ["user", "assistant"]) await inspector.getByRole("button", { name: `Add ${role} turn`, exact: true }).click();
      await expectField(inspector, "user turn 1", "textbox", "textarea");
      await expectField(inspector, "assistant turn 2", "textbox", "textarea");
    });

    test("lore fields have visible accessible labels and semantic controls", async ({ page }) => {
      await page.getByRole("button", { name: "Manuscript", exact: true }).click();
      const manuscript = page.locator("#manuscript-drawer");
      await manuscript.getByRole("region", { name: "Series lore", exact: true }).getByRole("button", { name: "New lore", exact: true }).click();
      const lore = page.getByRole("dialog", { name: "Create lore", exact: true });
      await expect(lore).toBeVisible();
      const fields = [
        ["Kind", "combobox", "select"],
        ["Name", "textbox", "input", "text"],
        ["Keywords (one per line)", "textbox", "textarea"],
        ["Aliases (one per line)", "textbox", "textarea"],
        ["Visible text (AI context)", "textbox", "textarea"],
        ["Hidden notes — author-only, never sent to AI", "textbox", "textarea"],
        ["In AI requests", "combobox", "select"],
      ] as const;
      for (const [name, role, tag, type] of fields) await expectField(lore, name, role, tag, type);
    });
  });
}
