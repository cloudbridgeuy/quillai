import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, click, render, type } from "../../test/dom";
import { ManuscriptTree } from "./ManuscriptTree";

const originalFetch = globalThis.fetch;
const series = { id: 1, title: "Original", position: 0 };
const tree = { series: [{ ...series, books: [] }], universes: [{ id: 2, title: "Universe", position: 0, containers: [{ ...series, universeId: 2, kind: "book_series", pieces: [] }], loose: [] }] };
afterEach(async () => { await cleanup(); globalThis.fetch = originalFetch; });

async function setup() {
  const puts: RequestInit[] = [];
  globalThis.fetch = Object.assign(async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === "PUT") { puts.push(init); return Response.json({ ...series, title: "Changed" }); }
    return Response.json(tree);
  }, { preconnect: originalFetch.preconnect });
  const container = await render(<ManuscriptTree selectedChapter={null} disabled={false} onSelect={() => {}} />);
  await click(container.querySelector<HTMLButtonElement>(`button[aria-label="Rename 'Original'"]`)!);
  const input = container.querySelector<HTMLInputElement>(`input[aria-label="Rename 'Original'"]`)!;
  await type(input, "Changed");
  return { container, input, puts };
}

test("Escape cancels the edited title without saving", async () => {
  const { container, input, puts } = await setup();
  await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(container.textContent).toContain("Original");
  expect(container.textContent).not.toContain("Changed");
  expect(puts).toHaveLength(0);
});

test("rename input is protected from the editor Escape handler", async () => {
  const { input } = await setup();
  expect(input.closest("dialog[open], [data-manuscript-draft]")).not.toBeNull();
});

test("blur saves the edited title exactly once", async () => {
  const { container, input, puts } = await setup();
  await act(async () => input.blur());
  expect(puts).toHaveLength(1);
  expect(JSON.parse(String(puts[0].body))).toEqual({ title: "Changed" });
  expect(container.textContent).toContain("Changed");
});

test("Enter saves the edited title", async () => {
  const { container, input, puts } = await setup();
  await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  expect(puts).toHaveLength(1);
  expect(JSON.parse(String(puts[0].body))).toEqual({ title: "Changed" });
  expect(container.textContent).toContain("Changed");
});
