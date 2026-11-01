import { expect, test } from "bun:test";
import type Quill from "quill";
import { continueOwnedRanges } from "./continue_stream";
import { editTintRemoval, renderContinueTint } from "./continue_tint";

test("Edit tint covers only the replacement when enabled and nonempty", () => {
  const removal = editTintRemoval(7, 14, true);
  expect(removal && continueOwnedRanges(removal)).toEqual([{ start: 7, end: 14 }]);
  expect(editTintRemoval(7, 7, true)).toBeNull();
  expect(editTintRemoval(7, 14, false)).toBeNull();
});

test("render marks only replacement endpoints and clears the range when disabled", () => {
  const globals = ["CSS", "Highlight", "document"].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)] as const);
  const highlights = new Map<string, Set<Range>>();
  const endpoints: [unknown, number][] = [];
  const firstNode = {};
  const lastNode = {};
  const lookups: number[] = [];
  const quill = {
    getLeaf: (index: number) => {
      lookups.push(index);
      return [{ position: (offset: number) => [index === 7 ? firstNode : lastNode, offset] }, index === 7 ? 2 : 4];
    },
  };
  Object.defineProperties(globalThis, {
    CSS: { configurable: true, value: { highlights } },
    Highlight: { configurable: true, value: class extends Set<Range> {
      constructor(...ranges: Range[]) { super(ranges); }
    } },
    document: { configurable: true, value: { createRange: () => ({
      setStart: (node: unknown, offset: number) => endpoints.push([node, offset]),
      setEnd: (node: unknown, offset: number) => endpoints.push([node, offset]),
    }) } },
  });
  try {
    const removal = editTintRemoval(7, 14, true)!;
    renderContinueTint(quill as unknown as Quill, removal, true);
    expect(lookups).toEqual([7, 14]);
    expect(endpoints).toEqual([[firstNode, 2], [lastNode, 4]]);
    expect(highlights.get("quillai-stream")?.size).toBe(1);
    renderContinueTint(quill as unknown as Quill, removal, false);
    expect(highlights.get("quillai-stream")?.size).toBe(0);
    expect(lookups).toEqual([7, 14]);
    for (const unavailable of ["CSS", "Highlight"]) {
      const descriptor = Object.getOwnPropertyDescriptor(globalThis, unavailable)!;
      Reflect.deleteProperty(globalThis, unavailable);
      expect(() => renderContinueTint(quill as unknown as Quill, removal, true)).not.toThrow();
      Object.defineProperty(globalThis, unavailable, descriptor);
    }
    Object.defineProperty(globalThis, "CSS", { configurable: true, value: {} });
    expect(() => renderContinueTint(quill as unknown as Quill, removal, true)).not.toThrow();
    expect(lookups).toEqual([7, 14]);
  } finally {
    for (const [name, descriptor] of globals) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else Reflect.deleteProperty(globalThis, name);
    }
  }
});
