import { expect, test, type APIRequestContext, type Locator, type Page } from "../app/node_modules/@playwright/test/index.js";
import { expectEditorReady } from "./editor-ready";

async function runContinue(page: Page, request: APIRequestContext, width: number) {
  const api = process.env.QUILLAI_E2E_API_URL;
  const provider = process.env.QUILLAI_E2E_FAKE_AI_URL;
  if (!api || !provider) throw new Error("Run with cargo xtask e2e and its default catalog");
  const identity = `GUZ868-1-${crypto.randomUUID()}`;
  const post = async (url: string, data: object) => {
    const response = await request.post(url, { data });
    expect(response.status()).toBe(201);
    return response.json();
  };
  const series = await post(`${api}/series`, { title: "Inspector cost", position: 0 });
  const book = await post(`${api}/series/${series.id}/books`, { title: "Book", position: 0 });
  const chapter = await post(`${api}/series/${series.id}/books/${book.id}/chapters`, {
    title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: `A quiet harbour. ${identity}\n` }] },
  });
  await page.setViewportSize({ width, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: String(series.id), bookId: String(book.id), chapterId: String(chapter.id) })}`);
  await expectEditorReady(page.locator(".ql-editor"));
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  await page.getByRole("button", { name: "Context inspector", exact: true }).click();
  const inspector = page.locator("#context-drawer");
  await inspector.getByText("Advanced AI controls", { exact: true }).click();
  const model = inspector.getByRole("combobox", { name: "Continue model", exact: true });
  await model.selectOption("e2e-continue");
  const before = await (await request.get(`${provider}/control`)).json();
  expect((await request.post(`${provider}/control/arm`, { data: { identity, model: "e2e-continue" } })).status()).toBe(200);
  await page.getByRole("button", { name: /^Continue writing/ }).click();
  const upstream = await (await request.get(`${provider}/control/${before.length}/request`)).json();
  expect(upstream.identity).toBe(identity);
  expect(upstream.model).toBe("e2e-continue");
  const control = `${provider}/control/${upstream.id}`;
  expect((await request.post(`${control}/fragment`, { data: { text: "The lantern flickered." } })).status()).toBe(200);
  expect((await request.post(`${control}/usage`)).status()).toBe(200);
  expect((await request.post(`${control}/finish`, { data: { reason: "stop" } })).status()).toBe(200);
  await expect(page.getByRole("button", { name: "Stop AI", exact: true })).toBeHidden();
  await expect(page.locator(".ql-editor")).toContainText("The lantern flickered.");
  await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
  return { model, cost: inspector.getByRole("region", { name: "Model and cost", exact: true }) };
}

async function expectUnbrokenWords(elements: Locator, minimum: number) {
  expect(await elements.count()).toBeGreaterThanOrEqual(minimum);
  for (const element of await elements.all()) {
    await expect(element).toBeVisible();
    const broken = await element.evaluate(node => {
      const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
      const words: string[] = [];
      while (walker.nextNode()) {
        const textNode = walker.currentNode;
        const text = textNode.textContent ?? "";
        for (const match of text.matchAll(/\S+/g)) {
          const range = document.createRange();
          range.setStart(textNode, match.index!);
          range.setEnd(textNode, match.index! + match[0].length);
          const tops = Array.from(range.getClientRects(), rect => Math.round(rect.top));
          if (new Set(tops).size > 1) words.push(match[0]);
        }
      }
      return words;
    });
    expect(broken, `broken words in ${await element.innerText()}`).toEqual([]);
  }
}

async function expectCostWords(cost: Locator) {
  await expect(cost.locator("dl")).toHaveCount(2);
  await expectUnbrokenWords(cost.locator("h1, h2, h3, h4, h5, h6"), 2);
  await expectUnbrokenWords(cost.locator("dl dt"), 5);
  await expectUnbrokenWords(cost.locator("dl dd"), 5);
}

function valueFor(cost: Locator, label: string) {
  return cost.locator("dt").filter({ hasText: new RegExp(`^${label}$`) }).locator("+ dd");
}

for (const width of [1100, 1440]) {
  test(`${width}px cost region shows one label when selected and request models match`, async ({ page, request }) => {
    const { cost } = await runContinue(page, request, width);
    await cost.scrollIntoViewIfNeeded();
    await expectCostWords(cost);
    const selected = valueFor(cost, "Model");
    await expect(selected).toHaveText("E2E Continue");
    await expect(selected).toHaveAttribute("title", "e2e-continue");
    await expect(cost.locator("dt").filter({ hasText: /^Request model$/ })).toHaveCount(0);
    expect((await cost.innerText()).match(/E2E Continue/g)).toHaveLength(1);
  });

  test(`${width}px cost region shows selected and request models when they differ`, async ({ page, request }) => {
    const { model, cost } = await runContinue(page, request, width);
    await model.selectOption("e2e-repeat");
    await cost.scrollIntoViewIfNeeded();
    await expectCostWords(cost);
    const selected = valueFor(cost, "Model");
    await expect(selected).toHaveText("E2E Repeat");
    await expect(selected).toHaveAttribute("title", "e2e-repeat");
    await expect(valueFor(cost, "Request model")).toContainText("E2E Continue");
    expect((await cost.innerText()).match(/E2E Repeat/g)).toHaveLength(1);
    expect((await cost.innerText()).match(/E2E Continue/g)).toHaveLength(1);
  });
}
