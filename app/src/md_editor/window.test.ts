import { expect, test } from "bun:test";

import { calculateDocumentWindow } from "./window";

test("cursor near the start clamps the window to zero", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 100,
      selection: { index: 2, length: 0 },
      windowSize: 20,
    }),
  ).toEqual({ start: 0, end: 20 });
});

test("cursor near the end clamps the window to the document length", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 100,
      selection: { index: 98, length: 0 },
      windowSize: 20,
    }),
  ).toEqual({ start: 80, end: 100 });
});

test("non-empty selection stays inside the transmitted window", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 100,
      selection: { index: 40, length: 10 },
      windowSize: 20,
    }),
  ).toEqual({ start: 35, end: 55 });
});

test("whole-document mode always returns the full document span", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 100,
      selection: { index: 40, length: 10 },
      windowSize: 20,
      mode: "document",
    }),
  ).toEqual({ start: 0, end: 100 });
});

test("selection bounds clamp safely when they exceed the document", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 10,
      selection: { index: 9, length: 10 },
      windowSize: 4,
    }),
  ).toEqual({ start: 6, end: 10 });
});

test("missing selection defaults to the trailing document window", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 30,
      selection: null,
      windowSize: 8,
    }),
  ).toEqual({ start: 22, end: 30 });
});

test("zero-length documents always return an empty window", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 0,
      selection: { index: 5, length: 10 },
      windowSize: 20,
    }),
  ).toEqual({ start: 0, end: 0 });
});

test("non-finite inputs clamp safely to finite bounds", () => {
  expect(
    calculateDocumentWindow({
      documentLength: 10,
      selection: { index: Number.NaN, length: Number.POSITIVE_INFINITY },
      windowSize: Number.NaN,
    }),
  ).toEqual({ start: 0, end: 1 });
});
