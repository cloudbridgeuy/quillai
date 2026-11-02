import { expect, test } from "bun:test";
import { readCaretPosition, safeCaretPosition, writeCaretPosition } from "./caret_position";

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
