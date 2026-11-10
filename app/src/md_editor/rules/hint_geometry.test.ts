import { expect, test } from "bun:test";
import { hintSegments, type Bounds, type Measure } from "./hint_geometry";

const text = "alpha beta gamma delta";
const box = (left: number, top: number, width: number): Bounds => ({ left, top, width, height: 20, bottom: top + 20, right: left + width });

function grid(wrapAfter: number) {
  const calls: [number, number][] = [];
  const measure: Measure = (index, length) => {
    calls.push([index, length]);
    const word = text.slice(0, index).split(" ").length - 1;
    const line = word >= wrapAfter ? 1 : 0;
    const column = line === 0 ? word : word - wrapAfter;
    return box(column * 60, line * 30 + 10, length * 10);
  };
  return { measure, calls };
}

const everything = { top: -1000, bottom: 1000 };

test("a hint on one line is one segment spanning its words", () => {
  const { measure } = grid(9);
  expect(hintSegments({ start: 6, length: 10 }, text, measure, everything)).toEqual([{ left: 60, top: 10, width: 110, height: 20 }]);
});

test("a hint that wraps gets one segment per line", () => {
  const { measure } = grid(2);
  expect(hintSegments({ start: 6, length: 16 }, text, measure, everything)).toEqual([
    { left: 60, top: 10, width: 40, height: 20 },
    { left: 0, top: 40, width: 110, height: 20 },
  ]);
});

test("a hint outside the viewport is skipped after measuring only its two ends", () => {
  const { measure, calls } = grid(9);
  expect(hintSegments({ start: 6, length: 10 }, text, measure, { top: 500, bottom: 900 })).toEqual([]);
  expect(calls).toEqual([[6, 1], [15, 1]]);
});

test("a hint partly in the viewport is drawn", () => {
  const { measure } = grid(2);
  expect(hintSegments({ start: 6, length: 16 }, text, measure, { top: 35, bottom: 900 })).toHaveLength(2);
});

test("a hint whose ends cannot be measured draws nothing", () => {
  const { measure } = grid(9);
  expect(hintSegments({ start: 0, length: 5 }, text, (index, length) => (length === 1 ? null : measure(index, length)), everything)).toEqual([]);
});

test("a word that cannot be measured is skipped", () => {
  const { measure } = grid(9);
  expect(hintSegments({ start: 6, length: 10 }, text, (index, length) => (index === 6 && length === 4 ? null : measure(index, length)), everything)).toEqual([{ left: 120, top: 10, width: 50, height: 20 }]);
});
