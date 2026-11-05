import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { scriptBlockRepairs } from "./line_format_repair";

const heading = { "script-block": "scene-heading" };

describe("scriptBlockRepairs", () => {
  test("restores the heading type after an insert at the start of a heading line", () => {
    const expected = new Delta().insert("Before").insert("\n").insert("INT. ROOM").insert("\n", heading);
    const actual = new Delta().insert("Before").insert("\n").insert("INT. ROOM").insert("\n");
    expect(scriptBlockRepairs(expected.ops, actual.ops)).toEqual([{ index: 16, value: "scene-heading" }]);
  });

  test("returns nothing when the formats agree", () => {
    const doc = new Delta().insert("INT. ROOM").insert("\n", heading);
    expect(scriptBlockRepairs(doc.ops, doc.ops)).toEqual([]);
  });

  test("reports null when the attribute should be absent", () => {
    const expected = new Delta().insert("a\n");
    const actual = new Delta().insert("a").insert("\n", heading);
    expect(scriptBlockRepairs(expected.ops, actual.ops)).toEqual([{ index: 1, value: null }]);
  });

  test("ignores other attributes", () => {
    const expected = new Delta().insert("a").insert("\n", { align: "center", "script-block": "action" });
    const actual = new Delta().insert("a").insert("\n", { "script-block": "action" });
    expect(scriptBlockRepairs(expected.ops, actual.ops)).toEqual([]);
  });
});
