import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { appendContinueFragment, continuePlacement, createContinueStream, transformContinueSelection, transformContinueStream } from "./continue_stream";

test("Continue chooses paragraph placement only for blank lines and completed sentences at line end", () => {
  for (const [text, caret, anchor, paragraph] of [
    ["\n", 0, 0, true],
    ["  \n", 1, 2, true],
    ["Hello.  \n", 6, 8, true],
    ["Hello!”\n", 7, 7, true],
    ["Hello?'\n", 7, 7, true],
    ["Hello. later\n", 6, 6, false],
    ["Hello there\n", 5, 5, false],
    ["Hello\n", 5, 5, false],
    ["Hello\nWorld\n", 6, 6, false],
  ] as const) {
    expect(continuePlacement(text, caret, "AI").paragraph).toBe(paragraph);
    expect(createContinueStream(text, caret)).toMatchObject({ anchor, paragraph });
  }
});

test("inline first fragment adds space only when needed and rolls it back with AI text", () => {
  for (const [text, caret, fragment, expected, space] of [
    ["Mara walked home.\n", 11, "slowly", "Mara walked slowly home.\n", true],
    ["Mara\n", 2, "x", "Maxra\n", false],
    ["Mara \n", 5, "walked", "Mara walked\n", false],
    ["Mara\n", 4, " walked", "Mara walked\n", false],
    ["Mara\n", 4, ", yes", "Mara, yes\n", false],
    ["Mara\n", 4, "—yes", "Mara—yes\n", false],
    ["Mara\n", 0, "Hello", "HelloMara\n", false],
  ] as const) {
    expect(continuePlacement(text, caret, fragment).space).toBe(space);
    const original = new Delta().insert(text);
    const appended = appendContinueFragment(createContinueStream(text, caret), new Delta().insert(fragment), text);
    const changed = original.compose(appended.change);
    expect(changed.ops).toEqual(new Delta().insert(expected).ops);
    expect(changed.compose(appended.state.removal).ops).toEqual(original.ops);
  }
});

test("first fragment uses author edits at the transformed anchor and later fragments add no separator", () => {
  const original = new Delta().insert("Mara walked home.\n");
  const edit = new Delta().retain(11).insert(" ");
  const state = transformContinueStream(createContinueStream("Mara walked home.\n", 11), edit);
  const document = original.compose(edit);
  const first = appendContinueFragment(state, new Delta().insert("slow"), "Mara walked  home.\n");
  expect(first.change.ops).toEqual([{ retain: 12 }, { insert: "slow" }]);
  const last = appendContinueFragment(first.state, new Delta().insert("ly"), "Mara walked slow home.\n");
  expect(last.change.ops).toEqual([{ retain: 16 }, { insert: "ly" }]);
  expect(document.compose(first.change).compose(last.change).compose(last.state.removal).ops).toEqual(document.ops);
});

test("Continue anchors below the original caret paragraph and follows author edits before arrival", () => {
  let state = createContinueStream("First paragraph.\nLast\n", 16);
  expect(state.anchor).toBe(16);
  state = transformContinueStream(state, new Delta().retain(2).insert("🦊"));
  const next = appendContinueFragment(state, new Delta().insert("AI"), "Fi🦊rst paragraph.\nLast\n");
  expect(next.change.ops).toEqual([{ retain: 18 }, { insert: "\nAI" }]);
  expect(next.state.end).toBe(21);
});

test("rollback deletes only surviving AI ranges after interior inserts, deletes, replacement and formatting", () => {
  let document = new Delta().insert("Before.\nAfter\n");
  let state = createContinueStream("Before.\nAfter\n", 7);
  const first = appendContinueFragment(state, new Delta().insert("abcdef"), "Before.\nAfter\n");
  state = first.state;
  document = document.compose(first.change);
  for (const change of [
    new Delta().retain(10).insert("AUTHOR"),
    new Delta().retain(8).delete(1),
    new Delta().retain(16).insert("R").delete(2),
    new Delta().retain(9).retain(6, { bold: true }),
  ]) {
    state = transformContinueStream(state, change);
    document = document.compose(change);
  }
  const last = appendContinueFragment(state, new Delta().insert("tail"), "");
  expect(state.end).toBeLessThan(last.state.end);
  expect(state.removal.ops).not.toEqual(last.state.removal.ops);
  document = document.compose(last.change);
  expect(document.compose(last.state.removal).ops).toEqual(
    new Delta().insert("Before.").insert("AUTHOR", { bold: true }).insert("R\nAfter\n").ops,
  );
});

test("boundary author inserts and complete AI deletion preserve ownership and append position", () => {
  const first = appendContinueFragment(createContinueStream("A.\nB\n", 2), new Delta().insert("AI"), "A.\nB\n");
  let state = transformContinueStream(first.state, new Delta().retain(2).insert("left"));
  state = transformContinueStream(state, new Delta().retain(state.end).insert("right"));
  expect(state.removal.ops).toEqual([{ retain: 6 }, { delete: 3 }]);
  state = transformContinueStream(state, new Delta().retain(6).delete(3));
  expect(state.removal.ops).toEqual([]);
  const next = appendContinueFragment(state, new Delta().insert("more"), "");
  expect(next.change.ops).toEqual([{ retain: 11 }, { insert: "more" }]);
});

test("stream inserts preserve the logical caret and selection without advancing at equal positions", () => {
  const change = new Delta().retain(5).insert("AI");
  expect(transformContinueSelection({ index: 5, length: 0 }, change)).toEqual({ index: 5, length: 0 });
  expect(transformContinueSelection({ index: 8, length: 4 }, change)).toEqual({ index: 10, length: 4 });
  expect(transformContinueSelection({ index: 2, length: 9 }, change)).toEqual({ index: 2, length: 11 });
});

test("deleting the whole saved selection collapses it at the deletion start", () => {
  expect(transformContinueSelection({ index: 5, length: 6 }, new Delta().retain(3).delete(10))).toEqual({ index: 3, length: 0 });
});

test("replacing the saved selection collapses it at the replacement end", () => {
  expect(transformContinueSelection({ index: 5, length: 6 }, new Delta().retain(5).insert("NEW").delete(6))).toEqual({ index: 8, length: 0 });
});

test("partial deletion selects exactly the undeleted remainder", () => {
  for (const [change, expected] of [
    [new Delta().retain(3).delete(5), { index: 3, length: 3 }],
    [new Delta().retain(8).delete(5), { index: 5, length: 3 }],
    [new Delta().retain(7).delete(2), { index: 5, length: 4 }],
  ] as const) {
    expect(transformContinueSelection({ index: 5, length: 6 }, change)).toEqual(expected);
  }
});
