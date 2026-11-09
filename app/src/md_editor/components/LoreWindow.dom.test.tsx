import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, click, render } from "../../test/dom";
import { LoreWindow } from "./LoreWindow";

afterEach(cleanup);

test("each click scrolls to the next match, wraps, and keeps the card open", async () => {
  const entry = {
    id: 1, name: "Mira", kind: "character", level: "include", reason: "matched", tokens: 1, visible_text: "The navigator.",
    occurrences: [0, 10, 20].map(start => ({ id: 1, start, end: start + 4, text: "Mira" })),
  };
  const lore = { injected: [entry], skipped: [], matches: [entry], maxCharacters: 100 } as never;
  const text = document.createTextNode("x".repeat(40));
  const root = document.createElement("div");
  root.append(text);
  document.body.append(root);
  const quill = {
    root, getSelection: () => null,
    getLeaf: (index: number) => [{ position: (offset: number) => [text, offset] }, index],
  } as never;
  const original = Range.prototype.getBoundingClientRect;
  Range.prototype.getBoundingClientRect = function (this: Range) {
    return { top: this.startOffset * 100, bottom: this.startOffset * 100 + 10, left: 0, right: 1, width: 1, height: 10, x: 0, y: 0, toJSON() {} };
  };
  const scrolled: number[] = [];
  const originalScroll = window.scrollBy;
  window.scrollBy = ((options: ScrollToOptions) => { scrolled.push(options.top ?? 0); }) as never;
  try {
    const container = await render(<LoreWindow quill={quill} start={0} lore={lore} />);
    const button = container.querySelector("button")!;
    const offset = window.innerHeight / 2;
    for (const start of [0, 10, 20, 0]) {
      await click(button);
      expect(scrolled.at(-1)).toBe(start * 100 - offset);
      expect(document.querySelector('[role="dialog"]')).not.toBeNull();
    }
    expect(scrolled).toHaveLength(4);
  } finally {
    Range.prototype.getBoundingClientRect = original;
    window.scrollBy = originalScroll;
    root.remove();
  }
});

test("window scroll rebuilds the marks only while a card is open", async () => {
  const entry = {
    id: 1, name: "Mira", kind: "character", level: "include", reason: "matched", tokens: 1, visible_text: "The navigator.",
    occurrences: [{ id: 1, start: 0, end: 4, text: "Mira" }],
  };
  const lore = { injected: [entry], skipped: [], matches: [entry], maxCharacters: 100 } as never;
  const text = document.createTextNode("x".repeat(40));
  const root = document.createElement("div");
  root.append(text);
  document.body.append(root);
  let leafLookups = 0;
  const quill = {
    root, getSelection: () => null,
    getLeaf: (index: number) => { leafLookups += 1; return [{ position: (offset: number) => [text, offset] }, index]; },
  } as never;
  const original = Range.prototype.getBoundingClientRect;
  Range.prototype.getBoundingClientRect = () => ({ top: 0, bottom: 10, left: 0, right: 1, width: 1, height: 10, x: 0, y: 0, toJSON() {} });
  const originalScroll = window.scrollBy;
  window.scrollBy = (() => {}) as never;
  const lookupsDuringScroll = async () => {
    const before = leafLookups;
    await act(async () => { window.dispatchEvent(new Event("scroll")); });
    return leafLookups - before;
  };
  try {
    const container = await render(<LoreWindow quill={quill} start={0} lore={lore} />);
    expect(await lookupsDuringScroll()).toBe(0);
    await click(container.querySelector("button")!);
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
    expect(await lookupsDuringScroll()).toBeGreaterThan(0);
  } finally {
    Range.prototype.getBoundingClientRect = original;
    window.scrollBy = originalScroll;
    root.remove();
  }
});
