import { expect, test } from "bun:test";
import { movePoint, nextPosition, orderPoints, pickerOptions } from "./timeline_panel";

const a = { id: 1, name: "A", position: 2 };
const b = { id: 2, name: "B", position: 0 };
const c = { id: 3, name: "C", position: 2 };

test("orders points by position then id", () => {
  expect(orderPoints([a, b, c]).map(point => point.id)).toEqual([2, 1, 3]);
});

test("next position follows the last point", () => {
  expect(nextPosition([])).toBe(0);
  expect(nextPosition([a, b])).toBe(3);
});

test("moving renumbers only the changed points", () => {
  expect(movePoint([b, a], 2, "down")).toEqual([{ id: 1, position: 0 }, { id: 2, position: 1 }]);
  expect(movePoint([a, b, c], 3, "up")).toEqual([{ id: 3, position: 1 }]);
});

test("moving past an end changes nothing", () => {
  expect(movePoint([b, a], 2, "up")).toEqual([]);
  expect(movePoint([b, a], 1, "down")).toEqual([]);
  expect(movePoint([b, a], 99, "up")).toEqual([]);
});

test("picker offers base text first, then points in order", () => {
  expect(pickerOptions([a, b], "No point (base text)")).toEqual([
    { value: "", label: "No point (base text)" }, { value: "2", label: "B" }, { value: "1", label: "A" },
  ]);
});
