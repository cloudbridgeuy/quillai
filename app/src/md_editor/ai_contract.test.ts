import { expect, test } from "bun:test";

import {
  buildAiGenerateUrl,
  buildAiRequest,
  DEFAULT_API_BASE_URL,
  resolveApiBaseUrl,
} from "./ai_contract";

const documentSlice = {
  delta: { ops: [{ insert: "Opening line." }] },
  text: "Opening line.",
};

test("buildAiRequest creates a generate payload with null optional fields", () => {
  expect(
    buildAiRequest({
      action: "generate",
      document: documentSlice,
      window: { start: 0, end: 13 },
      selection: null,
      storyContext: "   ",
      instruction: undefined,
    }),
  ).toEqual({
    action: "generate",
    document: documentSlice,
    selection: null,
    storyContext: null,
    instruction: null,
  });
});

test("buildAiRequest creates an edit payload with selection relative to the window", () => {
  expect(
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18, length: 4 },
      },
      storyContext: " fantasy  ",
      instruction: " tighten prose ",
    }),
  ).toEqual({
    action: "edit",
    document: documentSlice,
    selection: {
      delta: { ops: [{ insert: "line" }] },
      text: "line",
      range: { index: 8, length: 4 },
    },
    storyContext: "fantasy",
    instruction: "tighten prose",
  });
});

test("buildAiRequest rejects document and selection embeds", () => {
  expect(() =>
    buildAiRequest({
      action: "generate",
      document: {
        delta: { ops: [{ insert: { image: "x" } }] },
        text: "",
      },
      window: { start: 0, end: 0 },
      selection: null,
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("AI document slices currently support text inserts only");

  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 0, end: 13 },
      selection: {
        delta: { ops: [{ insert: { image: "x" } }] },
        text: "",
        range: { index: 0, length: 1 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("AI selection slices currently support text inserts only");
});

test("buildAiRequest preserves UTF-16 selection ranges for emoji text", () => {
  expect(
    buildAiRequest({
      action: "edit",
      document: {
        delta: { ops: [{ insert: "Hi 💡 there" }] },
        text: "Hi 💡 there",
      },
      window: { start: 0, end: 10 },
      selection: {
        delta: { ops: [{ insert: "💡" }] },
        text: "💡",
        range: { index: 3, length: 2 },
      },
      storyContext: null,
      instruction: "tighten",
    }),
  ).toEqual({
    action: "edit",
    document: {
      delta: { ops: [{ insert: "Hi 💡 there" }] },
      text: "Hi 💡 there",
    },
    selection: {
      delta: { ops: [{ insert: "💡" }] },
      text: "💡",
      range: { index: 3, length: 2 },
    },
    storyContext: null,
    instruction: "tighten",
  });
});

test("buildAiRequest rejects edit payloads without a selection", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 0, end: 13 },
      selection: null as never,
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("edit requests require a selection");
});

test("buildAiRequest rejects selections that are not fully contained in the transmitted window", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "ing li" }] },
        text: "ing li",
        range: { index: 8, length: 6 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection must be fully contained within the transmitted document window");
});

test("buildAiRequest rejects selections that exceed the transmitted document slice when window metadata is stale", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
      },
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18, length: 4 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection must fit within the transmitted document slice");
});

test("buildAiRequest rejects negative selection ranges", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: -1, length: 4 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection index must be a finite non-negative integer");

  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18, length: -1 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection length must be a finite non-negative integer");
});

test("buildAiRequest rejects fractional selection ranges", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18.5, length: 4 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection index must be a finite non-negative integer");

  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18, length: 4.5 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection length must be a finite non-negative integer");
});

test("buildAiRequest rejects NaN selection ranges", () => {
  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: Number.NaN, length: 4 },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection index must be a finite non-negative integer");

  expect(() =>
    buildAiRequest({
      action: "edit",
      document: documentSlice,
      window: { start: 10, end: 23 },
      selection: {
        delta: { ops: [{ insert: "line" }] },
        text: "line",
        range: { index: 18, length: Number.NaN },
      },
      storyContext: null,
      instruction: null,
    }),
  ).toThrow("selection length must be a finite non-negative integer");
});

test("resolveApiBaseUrl returns the local API default", () => {
  expect(resolveApiBaseUrl()).toBe(DEFAULT_API_BASE_URL);
});

test("buildAiGenerateUrl appends the AI route to the resolved base URL", () => {
  expect(buildAiGenerateUrl()).toBe("http://127.0.0.1:2908/ai/generate");
});
