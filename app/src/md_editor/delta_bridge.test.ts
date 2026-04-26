import { expect, test } from "bun:test";

import { toContractDeltaDocument, toQuillDeltaDocument } from "./delta_bridge";

test("toContractDeltaDocument keeps supported fields from Quill-like deltas", () => {
  expect(
    toContractDeltaDocument({
      ops: [
        { insert: "Hello", attributes: { bold: true, ignored: { nested: true } } },
        { retain: 2, attributes: { color: "red" } },
      ],
    }),
  ).toEqual({
    ops: [
      { insert: "Hello", attributes: { bold: true } },
      { retain: 2, attributes: { color: "red" } },
    ],
  });
});

test("toContractDeltaDocument drops unsupported Quill retain payload shapes", () => {
  expect(
    toContractDeltaDocument({
      ops: [{ retain: { unsupported: true } }],
    }),
  ).toEqual({
    ops: [],
  });
});

test("toQuillDeltaDocument removes null attributes for Quill consumption", () => {
  expect(
    toQuillDeltaDocument({
      ops: [{ insert: "Hello", attributes: { bold: true, link: null } }],
    }),
  ).toEqual({
    ops: [{ insert: "Hello", attributes: { bold: true } }],
  });
});
