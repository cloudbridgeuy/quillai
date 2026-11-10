import { expect, test } from "bun:test";
import Delta from "quill-delta";

import { appendContinueFragment, transformContinueStream, type ContinueStream } from "./continue_stream";
import { createDialogueStream, dialogueCaret, dialogueRunRequest, feedDialogue, finishDialogue, type DialogueFeed } from "./dialogue_stream";
import type { LoreMatchEntry } from "./lore_match";

const plain = (doc: Delta) => doc.ops.map(op => (typeof op.insert === "string" ? op.insert : "")).join("");

function runner(doc: Delta, caret: number, script: Record<string, unknown> | null = null) {
  const created = createDialogueStream(plain(doc), caret, script);
  let state: ContinueStream = created.stream;
  let feed: DialogueFeed = created.feed;
  const apply = (fragment: Delta) => {
    const appended = appendContinueFragment(state, fragment, plain(doc));
    state = appended.state;
    doc = doc.compose(appended.change);
  };
  return {
    push(chunk: string) {
      const fed = feedDialogue(feed, chunk);
      feed = fed.feed;
      apply(fed.fragment);
    },
    finish: () => apply(finishDialogue(feed)),
    author(change: Delta) {
      doc = doc.compose(change);
      state = transformContinueStream(state, change);
    },
    rollback: () => doc.compose(state.removal),
    doc: () => doc,
  };
}

test("prose dialogue at a mid-line caret splits the line into a new paragraph at the caret", () => {
  const run = runner(new Delta().insert("She waited.\nNext\n"), 3);
  run.push("\"Go,\" ");
  run.push("said Ana.\n\n\"No.\"");
  run.finish();
  expect(plain(run.doc())).toBe("She\n\"Go,\" said Ana.\n\n\"No.\"\n waited.\nNext\n");
});

test("prose dialogue at a line end starts a new paragraph with no empty line after it", () => {
  const run = runner(new Delta().insert("She waited.\nNext\n"), 11);
  run.push("\"Go,\" said Ana.");
  run.finish();
  expect(plain(run.doc())).toBe("She waited.\n\"Go,\" said Ana.\nNext\n");
});

test("a non-empty selection collapses to its end", () => {
  expect(dialogueCaret({ index: 2, length: 3 }, 10)).toBe(5);
  expect(dialogueCaret({ index: 4, length: 0 }, 10)).toBe(4);
  expect(dialogueCaret(null, 10)).toBe(10);
});

test("prose dialogue on an empty line fills that line", () => {
  const run = runner(new Delta().insert("A\n\nB\n"), 2);
  run.push("\"Hi.\"");
  run.finish();
  expect(plain(run.doc())).toBe("A\n\"Hi.\"\nB\n");
});

test("screenplay dialogue at a mid-line caret inserts character, parenthetical and dialogue blocks at the caret", () => {
  const block = (type: string) => ({ "script-block": type });
  const run = runner(new Delta().insert("Hello.").insert("\n", block("dialogue")), 2, block("dialogue"));
  run.push("ANA\n(quietly)\nGo");
  run.push(" now.\nBOB\nNo.");
  run.finish();
  expect(run.doc()).toEqual(new Delta()
    .insert("He").insert("\n", block("dialogue"))
    .insert("ANA").insert("\n", block("character"))
    .insert("(quietly)").insert("\n", block("parenthetical"))
    .insert("Go now.").insert("\n", block("dialogue"))
    .insert("BOB").insert("\n", block("character"))
    .insert("No.").insert("\n", block("dialogue"))
    .insert("llo.").insert("\n", block("dialogue")));
});

test("rollback removes only AI text and keeps the author's inserts", () => {
  const run = runner(new Delta().insert("Start\n"), 5);
  run.push("\"A.\"");
  run.author(new Delta().insert("X"));
  run.push("\n\"B.\"");
  run.author(new Delta().retain(plain(run.doc()).length - 1).insert("!"));
  expect(plain(run.doc())).toBe("XStart\n\"A.\"\n\"B.\"!\n");
  expect(plain(run.rollback())).toBe("XStart!\n");
});

test("a run that fails before any text, such as AI_NOT_CONFIGURED, has nothing to roll back", () => {
  const doc = new Delta().insert("Start\n");
  const { stream } = createDialogueStream(plain(doc), 5, null);
  expect(stream).toEqual({ anchor: 5, end: 5, started: false, paragraph: false, removal: new Delta() });
  expect(doc.compose(stream.removal)).toEqual(doc);
});

test("the request body holds speakers, brief and speaker lore first, with no hidden notes or model", () => {
  const entry = (id: number, name: string, extra: Partial<LoreMatchEntry> = {}) => ({
    id, kind: "character" as const, name, aliases: [], keywords: [name], visible_text: `${name} text`,
    pinned: false, level: "include" as const, hidden_notes: `${name} secret`, ...extra,
  });
  const text = "Bram looked at the Keep.";
  const body = dialogueRunRequest(
    { speakers: ["Zed", "Ana"], brief: " argue " },
    { request: { document: { delta: { ops: [{ insert: text }] }, text } }, window: { start: 0, end: text.length } },
    text.length,
    [entry(1, "Bram"), entry(2, "Zed"), entry(3, "Ana"), entry(4, "Keep", { kind: "place" })],
    " night ",
  );
  expect(body.action).toBe("dialogue");
  expect(body.dialogue).toEqual({ speakers: ["Zed", "Ana"], brief: "argue" });
  expect(body.lore.map(item => item.name)).toEqual(["Zed", "Ana", "Bram", "Keep"]);
  expect(body.storyContext).toBe("night");
  expect(JSON.stringify(body)).not.toContain("hidden_notes");
  expect(JSON.stringify(body)).not.toContain("secret");
  expect(Object.keys(body)).not.toContain("model");
});
