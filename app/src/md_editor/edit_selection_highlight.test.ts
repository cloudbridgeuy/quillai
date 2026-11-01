import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { editSelectionBounds, renderEditSelectionHighlight, trackEditSelectionHighlight } from "./edit_selection_highlight";
import type Quill from "quill";

test("edit selection bounds require a nonempty integer range inside the document", () => {
  expect(editSelectionBounds({ index: 0, length: 7 }, 12)).toEqual({ start: 0, end: 7 });
  expect(editSelectionBounds({ index: 5, length: 7 }, 12)).toEqual({ start: 5, end: 12 });
  for (const [selection, length] of [
    [null, 12], [{ index: -1, length: 1 }, 12], [{ index: 0, length: 0 }, 12],
    [{ index: 0, length: -1 }, 12], [{ index: 6, length: 7 }, 12],
    [{ index: 0.5, length: 1 }, 12], [{ index: 0, length: 1.5 }, 12],
    [{ index: NaN, length: 1 }, 12], [{ index: 0, length: Infinity }, 12],
    [{ index: 0, length: 1 }, Infinity], [{ index: 0, length: 1 }, 1.5],
  ] as const) expect(editSelectionBounds(selection, length)).toBeNull();
});

test("render is safe without the CSS Highlight API", () => {
  expect(() => renderEditSelectionHighlight({} as Quill, { index: 0, length: 1 })).not.toThrow();
});

test("saved selection survives blur, follows user selection, and clears on all document changes and cleanup", () => {
  const globals = ["CSS", "Highlight", "document"].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)] as const);
  const highlights = new Map<string, Set<Range>>();
  const endpoints: number[] = [];
  Object.defineProperties(globalThis, {
    CSS: { configurable: true, value: { highlights } },
    Highlight: { configurable: true, value: class extends Set<Range> {
      constructor(...ranges: Range[]) { super(ranges); }
    } },
    document: { configurable: true, value: { createRange: () => ({
      setStart: (_node: unknown, offset: number) => endpoints.push(offset),
      setEnd: (_node: unknown, offset: number) => endpoints.push(offset),
    }) } },
  });
  const emitter = Object.assign(new EventEmitter(), {
    getLength: () => 12,
    getLeaf: (index: number) => [{ position: (offset: number) => [{}, offset] }, index],
  });
  const key = "quillai-edit-selection";
  try {
    const dispose = trackEditSelectionHighlight(emitter as unknown as Quill, { index: 0, length: 7 });
    expect(endpoints).toEqual([0, 7]);
    emitter.emit("selection-change", null, { index: 0, length: 7 }, "user");
    expect(highlights.has(key)).toBe(true);
    emitter.emit("selection-change", { index: 2, length: 3 }, null, "user");
    expect(endpoints.slice(-2)).toEqual([2, 5]);
    for (const source of ["silent", "api", "user"]) {
      emitter.emit("editor-change", "text-change", {}, {}, source);
      expect(highlights.has(key)).toBe(false);
      emitter.emit("selection-change", { index: 0, length: 7 }, null, "api");
      expect(highlights.has(key)).toBe(false);
      emitter.emit("selection-change", { index: 1, length: 2 }, null, "user");
      expect(highlights.has(key)).toBe(true);
    }
    emitter.emit("selection-change", { index: 1, length: 0 }, null, "user");
    expect(highlights.has(key)).toBe(false);
    emitter.emit("selection-change", { index: 1, length: 2 }, null, "user");
    dispose();
    expect(highlights.has(key)).toBe(false);
    expect(emitter.listenerCount("selection-change")).toBe(0);
    expect(emitter.listenerCount("editor-change")).toBe(0);
  } finally {
    for (const [name, descriptor] of globals) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else Reflect.deleteProperty(globalThis, name);
    }
  }
});
