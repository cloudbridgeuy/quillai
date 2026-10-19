import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { parseDelta, type Revision } from "../manuscripts/contract";
import { chapterDelta, editChapter, hasLocalChanges, receiveRevisions, sendChapter, startChapterSync, syncDocument } from "./chapter_sync";

const initial = () => startChapterSync({ ops: [{ insert: "text\n" }] }, 0);
const revision = (rev: number, delta: Delta): Revision => ({ rev, delta: parseDelta(delta) });

test("emoji insertion between shared high surrogates stays valid through pending, transform and acknowledgement", () => {
  const base = startChapterSync({ ops: [{ insert: "😀🚀\n" }] }, 0);
  const document = new Delta().insert("😀😀a🚀\n");
  const edited = editChapter(base, document);
  expect(edited.pending.ops.every((op) => typeof op.insert !== "string" || op.insert.isWellFormed())).toBe(true);
  expect(syncDocument(edited)).toEqual(document);
  const sent = sendChapter(edited)!;
  const pending = editChapter(sent.state, document.compose(new Delta().retain(5).insert("🌙")));
  const remote = revision(1, new Delta().insert("R"));
  const ack = revision(2, chapterDelta(remote.delta).transform(chapterDelta(sent.request.delta), true));
  const rebased = receiveRevisions(pending, [remote, ack], ack);
  const next = sendChapter(rebased)!;
  for (const request of [sent.request, next.request]) {
    expect(request.delta.ops.every((op) => typeof op.insert !== "string" || op.insert.isWellFormed())).toBe(true);
  }
  const pendingAck = revision(3, chapterDelta(next.request.delta));
  const local = receiveRevisions(next.state, [pendingAck], pendingAck);
  const other = receiveRevisions(base, [remote, ack, pendingAck]);
  expect(syncDocument(local)).toEqual(new Delta().insert("R😀😀a🌙🚀\n"));
  expect(syncDocument(other)).toEqual(syncDocument(local));
  expect(hasLocalChanges(local)).toBe(false);
});

test("loads server content and revision without local work", () => {
  const state = startChapterSync({ ops: [{ insert: "restored\n" }] }, 42);
  expect(state.rev).toBe(42);
  expect(syncDocument(state).ops).toEqual([{ insert: "restored\n" }]);
  expect(hasLocalChanges(state)).toBe(false);
  expect(sendChapter(state)).toBeNull();
});

test("composes pending edits and allows only one in-flight delta", () => {
  const original = initial();
  const first = editChapter(original, syncDocument(original).compose(new Delta().insert("A")));
  const submission = sendChapter(first)!;
  const second = editChapter(submission.state, syncDocument(submission.state).compose(new Delta().retain(1).insert("B")));
  const third = editChapter(second, syncDocument(second).compose(new Delta().retain(2).insert("C")));
  expect(submission.request).toEqual({ baseRev: 0, delta: { ops: [{ insert: "A" }] } });
  expect(sendChapter(third)).toBeNull();
  expect(third.pending.ops).toEqual([{ retain: 1 }, { insert: "BC" }]);
  expect(original).toEqual(initial());
});

test("acknowledges without applying the sent edit twice; new pending stays queued", () => {
  const local = editChapter(initial(), new Delta().insert("Atext\n"));
  const sent = sendChapter(local)!;
  const pending = editChapter(sent.state, new Delta().insert("ABtext\n"));
  const ack = revision(1, chapterDelta(sent.request.delta));
  const next = receiveRevisions(pending, [ack], ack);
  expect(syncDocument(next).ops).toEqual([{ insert: "ABtext\n" }]);
  expect(next.flight).toBeNull();
  expect(sendChapter(next)?.request).toEqual({ baseRev: 1, delta: { ops: [{ retain: 1 }, { insert: "B" }] } });
});

test("concurrent same-index inserts converge with server collision priority", () => {
  const base = initial();
  const a = sendChapter(editChapter(base, new Delta().insert("Atext\n")))!;
  const b = sendChapter(editChapter(base, new Delta().insert("Btext\n")))!;
  const first = revision(1, chapterDelta(a.request.delta));
  const second = revision(2, chapterDelta(first.delta).transform(chapterDelta(b.request.delta), true));
  const server = base.server.compose(chapterDelta(first.delta)).compose(chapterDelta(second.delta));
  const clientA = receiveRevisions(a.state, [first, second], first);
  const clientB = receiveRevisions(b.state, [first, second], second);
  expect(server.ops).toEqual([{ insert: "ABtext\n" }]);
  expect(syncDocument(clientA)).toEqual(server);
  expect(syncDocument(clientB)).toEqual(server);
  expect(hasLocalChanges(clientA)).toBe(false);
  expect(hasLocalChanges(clientB)).toBe(false);
});

test("remote before acknowledgement rebases flight and newly composed pending only once", () => {
  const sent = sendChapter(editChapter(initial(), new Delta().insert("Atext\n")))!;
  const pending = editChapter(sent.state, new Delta().insert("Aptext\n"));
  const remote = revision(1, new Delta().insert("R"));
  const ack = revision(2, chapterDelta(remote.delta).transform(chapterDelta(sent.request.delta), true));
  const after = revision(3, new Delta().retain(2).insert("S"));
  const next = receiveRevisions(pending, [remote, ack, after], ack);
  expect(syncDocument(next).ops).toEqual([{ insert: "RASptext\n" }]);
  expect(next.pending.ops).toEqual([{ retain: 3 }, { insert: "p" }]);
  expect(next.rev).toBe(3);
});

test("remote polling transforms unsent local work", () => {
  const pending = editChapter(initial(), new Delta().insert("Ltext\n"));
  const next = receiveRevisions(pending, [revision(1, new Delta().insert("R"))]);
  expect(syncDocument(next).ops).toEqual([{ insert: "RLtext\n" }]);
  expect(next.pending.ops).toEqual([{ retain: 1 }, { insert: "L" }]);
});

test("deletes, formatting, embeds and UTF-16 edits converge", () => {
  const base = startChapterSync(parseDelta(new Delta().insert("😀ab").insert({ image: "x" }).insert("\n")), 7);
  const localChange = new Delta().retain(2).delete(1).retain(1, { bold: true });
  const remoteChange = new Delta().retain(2).insert("R").retain(2).delete(1);
  const sent = sendChapter(editChapter(base, syncDocument(base).compose(localChange)))!;
  const remote = revision(8, remoteChange);
  const ack = revision(9, remoteChange.transform(localChange, true));
  const next = receiveRevisions(sent.state, [remote, ack], ack);
  expect(syncDocument(next)).toEqual(base.server.compose(remoteChange).compose(chapterDelta(ack.delta)));
  expect(hasLocalChanges(next)).toBe(false);
});

test("local undo that cancels pending work does not submit an empty delta", () => {
  const base = initial();
  const edited = editChapter(base, new Delta().insert("changed\n"));
  const undone = editChapter(edited, syncDocument(base));
  expect(hasLocalChanges(undone)).toBe(false);
  expect(sendChapter(undone)).toBeNull();
});

test("acknowledged duplicates are ignored on repeat polling", () => {
  const remote = revision(1, new Delta().insert("R"));
  const next = receiveRevisions(initial(), [remote]);
  expect(receiveRevisions(next, [remote])).toEqual(next);
});

test("history gaps, absent and mismatched acknowledgements preserve the input", () => {
  const sent = sendChapter(editChapter(initial(), new Delta().insert("Atext\n")))!;
  const snapshot = JSON.stringify(sent.state);
  const ack = revision(1, new Delta().insert("A"));
  expect(() => receiveRevisions(sent.state, [revision(2, new Delta().insert("R"))])).toThrow("Missing");
  expect(() => receiveRevisions(sent.state, [], ack)).toThrow("not in history");
  expect(() => receiveRevisions(sent.state, [revision(1, new Delta().insert("R"))], ack)).toThrow("mismatch");
  expect(JSON.stringify(sent.state)).toBe(snapshot);
});

test("AI replacement, fragments and rollback compose through the same local path", () => {
  const base = initial();
  const deleted = editChapter(base, new Delta().insert("\n"));
  const fragment = editChapter(deleted, new Delta().insert("new\n"));
  const restored = editChapter(fragment, syncDocument(base));
  expect(hasLocalChanges(restored)).toBe(false);
  expect(syncDocument(restored)).toEqual(syncDocument(base));
});

test("all insertion positions converge across remote, acknowledgement, and pending submission", () => {
  for (let localIndex = 0; localIndex < 5; localIndex++) {
    for (let remoteIndex = 0; remoteIndex < 5; remoteIndex++) {
      for (let pendingIndex = 0; pendingIndex < 6; pendingIndex++) {
        const base = initial();
        const local = new Delta().retain(localIndex).insert("L");
        const remote = new Delta().retain(remoteIndex).insert("R");
        const sent = sendChapter(editChapter(base, syncDocument(base).compose(local)))!;
        const pending = editChapter(sent.state, syncDocument(sent.state).compose(new Delta().retain(pendingIndex).insert("P")));
        const remoteRevision = revision(1, remote);
        const ack = revision(2, remote.transform(local, true));
        const rebased = receiveRevisions(pending, [remoteRevision, ack], ack);
        const next = sendChapter(rebased)!;
        const pendingAck = revision(3, chapterDelta(next.request.delta));
        const server = base.server.compose(remote).compose(chapterDelta(ack.delta)).compose(chapterDelta(pendingAck.delta));
        expect(syncDocument(receiveRevisions(next.state, [pendingAck], pendingAck))).toEqual(server);
        expect(syncDocument(receiveRevisions(base, [remoteRevision, ack, pendingAck]))).toEqual(server);
      }
    }
  }
});
