import { expect, test } from "bun:test";
import { treeScrollTarget } from "./tree_scroll";

test("tree scroll moves only enough to reveal the active row", () => {
  expect(treeScrollTarget(100, 120, 150, 100, 400)).toBe(100);
  expect(treeScrollTarget(100, 100, 400, 100, 400)).toBe(100);
  expect(treeScrollTarget(100, 70, 100, 100, 400)).toBe(66);
  expect(treeScrollTarget(100, 420, 450, 100, 400)).toBe(154);
});

test("tree scroll rounds fractional targets outward and leaves a four-pixel margin", () => {
  expect(treeScrollTarget(0, 1401.156, 1425.156, 45, 365)).toBe(1065);
  expect(treeScrollTarget(100, 69.844, 93.844, 100, 420)).toBe(65);
});

test("tree scroll includes creation rows when the block fits", () => {
  expect(treeScrollTarget(0, 400, 424, 45, 365, 488)).toBe(127);
  expect(treeScrollTarget(200, 20, 44, 45, 365, 108)).toBe(171);
  expect(treeScrollTarget(0, 100, 124, 45, 365, 300)).toBe(0);
});

test("tree scroll falls back to the active row when the block is too tall", () => {
  expect(treeScrollTarget(0, 400, 424, 45, 365, 800)).toBe(63);
  expect(treeScrollTarget(0, 400, 424, 45, 365, 713)).toBe(63);
});

test("tree scroll accounts for fractional block edges and rounded margin clearance", () => {
  expect(treeScrollTarget(0, 1401.156, 1425.156, 45, 365, 1489.156)).toBe(1129);
  expect(treeScrollTarget(0, 400.2, 424.2, 45.25, 365.25, 712.2)).toBe(63);
});
