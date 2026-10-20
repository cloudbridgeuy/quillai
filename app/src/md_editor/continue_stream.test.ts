import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { appendContinueFragment, createContinueStream, transformContinueSelection, transformContinueStream } from "./continue_stream";

test("Continue anchors below the original caret paragraph and follows author edits before arrival", () => {
  let state = createContinueStream("First paragraph\nLast\n", 3);
  expect(state.anchor).toBe(15);
  state = transformContinueStream(state, new Delta().retain(2).insert("🦊"));
  const next = appendContinueFragment(state, new Delta().insert("AI"));
  expect(next.change.ops).toEqual([{ retain: 17 }, { insert: "\nAI" }]);
  expect(next.state.end).toBe(20);
});

test("rollback deletes only surviving AI ranges after interior inserts, deletes, replacement and formatting", () => {
  let document = new Delta().insert("Before\nAfter\n");
  let state = createContinueStream("Before\nAfter\n", 2);
  const first = appendContinueFragment(state, new Delta().insert("abcdef"));
  state = first.state;
  document = document.compose(first.change);
  for (const change of [
    new Delta().retain(9).insert("AUTHOR"),
    new Delta().retain(7).delete(1),
    new Delta().retain(15).insert("R").delete(2),
    new Delta().retain(8).retain(6, { bold: true }),
  ]) {
    state = transformContinueStream(state, change);
    document = document.compose(change);
  }
  const last = appendContinueFragment(state, new Delta().insert("tail"));
  expect(state.end).toBeLessThan(last.state.end);
  expect(state.removal.ops).not.toEqual(last.state.removal.ops);
  document = document.compose(last.change);
  expect(document.compose(last.state.removal).ops).toEqual(
    new Delta().insert("Before").insert("AUTHOR", { bold: true }).insert("R\nAfter\n").ops,
  );
});

test("boundary author inserts and complete AI deletion preserve ownership and append position", () => {
  const first = appendContinueFragment(createContinueStream("A\nB\n", 0), new Delta().insert("AI"));
  let state = transformContinueStream(first.state, new Delta().retain(1).insert("left"));
  state = transformContinueStream(state, new Delta().retain(state.end).insert("right"));
  expect(state.removal.ops).toEqual([{ retain: 5 }, { delete: 3 }]);
  state = transformContinueStream(state, new Delta().retain(5).delete(3));
  expect(state.removal.ops).toEqual([]);
  const next = appendContinueFragment(state, new Delta().insert("more"));
  expect(next.change.ops).toEqual([{ retain: 10 }, { insert: "more" }]);
});

test("stream inserts preserve the logical caret and selection without advancing at equal positions", () => {
  const change = new Delta().retain(5).insert("AI");
  expect(transformContinueSelection({ index: 5, length: 0 }, change)).toEqual({ index: 5, length: 0 });
  expect(transformContinueSelection({ index: 8, length: 4 }, change)).toEqual({ index: 10, length: 4 });
  expect(transformContinueSelection({ index: 2, length: 9 }, change)).toEqual({ index: 2, length: 11 });
});
