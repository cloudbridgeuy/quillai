import { afterEach, expect, spyOn, test } from "bun:test";
import Quill from "quill";
import { anchorDelta, createCaretTracker, readCaretPosition, safeCaretPosition, writeCaretPosition } from "./caret_position";

test("validates and clamps both UTF-16 selection boundaries", () => {
  expect(safeCaretPosition({ index: 2, length: 2, offsetTop: 30 }, "a😀b\n")).toEqual({ index: 1, length: 3, offsetTop: 30 });
  expect(safeCaretPosition({ index: 1, length: 1, offsetTop: 0 }, "a😀b\n")).toEqual({ index: 1, length: 0, offsetTop: 0 });
  expect(safeCaretPosition({ index: 99, length: 9, offsetTop: 2 }, "\n")).toEqual({ index: 0, length: 0, offsetTop: 2 });
  expect(safeCaretPosition({ index: 1, length: 9, offsetTop: 2 }, "")).toEqual({ index: 0, length: 0, offsetTop: 2 });
});

test("rejects malformed and negative positions", () => {
  for (const value of [null, {}, { index: -1, length: 0, offsetTop: 0 }, { index: 0, length: -1, offsetTop: 0 }, { index: 0, length: 0, offsetTop: -1 }, { index: 0.5, length: 0, offsetTop: 0 }, { index: Infinity, length: 0, offsetTop: 0 }, { index: "1", length: 0, offsetTop: 0 }, { index: 0, length: 0.5, offsetTop: 0 }, { index: 0, length: 0, offsetTop: NaN }]) {
    expect(safeCaretPosition(value, "text\n")).toBeNull();
  }
});

test("stores scoped positions and tolerates broken or unavailable storage", () => {
  let data = "broken";
  const storage = { length: 1, clear() {}, key: () => null, removeItem() {}, getItem: () => data, setItem: (_key: string, value: string) => { data = value; } } satisfies Storage;
  const position = { index: 2, length: 1, offsetTop: 3 };
  expect(readCaretPosition("a", storage)).toBeNull();
  writeCaretPosition("a", position, storage);
  writeCaretPosition("b", { ...position, index: 4 }, storage);
  expect(readCaretPosition("a", storage)).toEqual(position);
  expect(readCaretPosition("b", storage)?.index).toBe(4);
  const throwing = { getItem: () => { throw Error(); }, setItem: () => { throw Error(); } } as unknown as Storage;
  expect(readCaretPosition("a", throwing)).toBeNull();
  expect(() => writeCaretPosition("a", position, throwing)).not.toThrow();
  expect(readCaretPosition("missing")).toBeNull();
  expect(() => writeCaretPosition("missing", position)).not.toThrow();
  for (const invalid of ["null", "[]", "42"]) { data = invalid; expect(readCaretPosition("a", storage)).toBeNull(); }
});

test("anchors only a caret line that was on screen", () => {
  expect(anchorDelta(437, 2054, 900)).toBe(1617);
  expect(anchorDelta(437, 300, 900)).toBe(-137);
  expect(anchorDelta(-50, 900, 900)).toBe(0);
  expect(anchorDelta(900, 1200, 900)).toBe(0);
});

const realObserver = globalThis.ResizeObserver;
let resize = () => {};
afterEach(() => {
  globalThis.ResizeObserver = realObserver;
  window.localStorage.clear();
  document.body.innerHTML = "";
});

function reflowEditor() {
  globalThis.ResizeObserver = class {
    constructor(callback: ResizeObserverCallback) { resize = () => callback([], this); }
    observe() {}
    unobserve() {}
    disconnect() { resize = () => {}; }
  };
  const host = document.createElement("div");
  document.body.append(host);
  const quill = new Quill(host);
  quill.setText("one\ntwo\nthree\n", "silent");
  const layout = { top: 437, width: 530 };
  quill.getBounds = () => ({ top: layout.top, bottom: layout.top + 20, left: 0, right: 10, height: 20, width: 10 });
  quill.container.getBoundingClientRect = () => new DOMRect(0, 0, 600, 4000);
  quill.root.getBoundingClientRect = () => new DOMRect(0, 0, layout.width, 4000);
  const scroll = spyOn(window, "scrollBy").mockImplementation(() => {});
  return { quill, layout, scroll };
}

test("keeps the on-screen caret line in place when the editor narrows", () => {
  const { quill, layout, scroll } = reflowEditor();
  const tracker = createCaretTracker(quill);
  quill.setSelection(5, 0, "user");
  layout.top = 2054;
  layout.width = 470;
  resize();
  expect(scroll).toHaveBeenCalledWith(0, 1617);
  tracker.close();
  scroll.mockRestore();
});

test("leaves a caret scrolled off screen and height-only changes alone", () => {
  const { quill, layout, scroll } = reflowEditor();
  const tracker = createCaretTracker(quill);
  quill.setSelection(5, 0, "user");
  layout.top = 600;
  resize();
  layout.top = -400;
  window.dispatchEvent(new Event("scroll"));
  layout.top = 900;
  layout.width = 470;
  resize();
  expect(scroll).not.toHaveBeenCalled();
  layout.top = 437;
  window.dispatchEvent(new Event("scroll"));
  layout.top = 500;
  layout.width = 530;
  resize();
  expect(scroll).toHaveBeenCalledWith(0, 63);
  tracker.close();
  scroll.mockRestore();
});

test("does not adjust while a chapter-switch restore is pending", () => {
  const { quill, layout, scroll } = reflowEditor();
  const tracker = createCaretTracker(quill);
  writeCaretPosition("chapter:1:2:3", { index: 5, length: 0, offsetTop: 437 });
  expect(tracker.open({ seriesId: 1, bookId: 2, chapterId: 3 })).toBe(true);
  scroll.mockClear();
  layout.top = 2054;
  layout.width = 470;
  resize();
  expect(scroll).not.toHaveBeenCalled();
  layout.top = 437;
  tracker.finish();
  scroll.mockClear();
  layout.top = 900;
  layout.width = 530;
  resize();
  expect(scroll).toHaveBeenCalledWith(0, 463);
  tracker.close();
  scroll.mockRestore();
});
