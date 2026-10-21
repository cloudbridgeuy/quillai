import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { parseDelta, type Revision, type SubmitDelta } from "../manuscripts/contract";
import { chapterDelta, editChapter, hasLocalChanges, receiveRevisions, sendChapter, startChapterSync, syncDocument } from "./chapter_sync";

const editCount = 400;
const replaySeed = process.env.QUILLAI_SYNC_SEED;
const seeds = replaySeed === undefined
  ? [0, 1, 2, 3, 7, 42, 729, 726, 20261021, 0x12345678, 0x80000000, 0xffffffff]
  : [Number(replaySeed)];

if (seeds.some((seed) => !Number.isSafeInteger(seed) || seed < 0 || seed > 0xffffffff) || replaySeed?.trim() === "") {
  throw new Error("QUILLAI_SYNC_SEED must be an unsigned 32-bit integer");
}

function seededRandom(seed: number) {
  let state = seed;
  return (limit: number) => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = Math.imul(state ^ (state >>> 15), state | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return Math.floor(((value ^ (value >>> 14)) >>> 0) / 0x100000000 * limit);
  };
}

function randomEdit(document: Delta, random: (limit: number) => number): Delta {
  const text = document.ops.map((op) => op.insert).join("");
  const boundaries = [0];
  for (const character of Array.from(text.slice(0, -1))) {
    boundaries.push(boundaries[boundaries.length - 1]! + character.length);
  }
  const start = random(boundaries.length);
  const index = boundaries[start]!;
  const end = Math.min(boundaries.length - 1, start + 1 + random(4));
  const length = boundaries[end]! - index;
  const change = new Delta().retain(index);
  const kind = random(4);
  const formats = [{ bold: true }, { italic: true }, { bold: null }, { italic: null }, { color: "#ff0000" }, { color: null }];
  if (kind === 0 || kind === 3 || length === 0) {
    const tokens = ["a", "bc", " ", "é", "中", "😀", "🚀", "🌙", "👩‍💻", "😀😀"];
    const insertFormats = [{ bold: true }, { italic: true }, { color: "#ff0000" }];
    change.insert(tokens[random(tokens.length)]!, random(2) ? insertFormats[random(insertFormats.length)] : undefined);
  }
  if ((kind === 1 || kind === 3) && length > 0) change.delete(length);
  if (kind === 2 && length > 0) change.retain(length, formats[random(formats.length)]);
  return change;
}

for (const seed of seeds) {
  test(`two clients converge after ${editCount} seeded edits; seed=${seed}`, () => {
    const random = seededRandom(seed);
    const initial = new Delta().insert("😀🚀ab", { bold: true }).insert(" café中🌙\n");
    const clients = [0, 1].map(() => ({
      state: startChapterSync(parseDelta(initial), 0),
      request: null as SubmitDelta | null,
      acknowledgement: null as Revision | null,
      edits: 0,
    }));
    const history: Revision[] = [];
    const documents = [initial];
    let server = initial;
    let edits = 0;
    let step = 0;
    const trace: unknown[] = [];
    const coverage = { remoteDuringFlight: 0, composedPending: 0, lateAck: 0, concurrentSubmission: 0 };

    try {
      while (edits < editCount || clients.some((client) => hasLocalChanges(client.state) || client.state.rev < history.length)) {
        if (step >= 10000) throw new Error("Scheduler did not drain");
        const actions: (() => void)[] = [];
        for (const [id, client] of clients.entries()) {
          if (edits < editCount) {
            const edit = () => {
              const before = syncDocument(client.state);
              const delta = randomEdit(before, random);
              trace.push({ step, id, edit: delta.ops });
              if (client.state.pending.ops.length > 0) coverage.composedPending++;
              const document = before.compose(delta);
              client.state = editChapter(client.state, document);
              expect(syncDocument(client.state)).toEqual(document);
              client.edits++;
              edits++;
            };
            actions.push(edit, edit, edit);
          }
          if (client.state.flight === null && client.state.pending.ops.length > 0) {
            actions.push(() => {
              const sent = sendChapter(client.state);
              if (!sent) throw new Error("Pending edit was not sent");
              trace.push({ step, id, send: sent.request });
              client.state = sent.state;
              client.request = sent.request;
            });
          }
          if (client.request) {
            actions.push(() => {
              const request = client.request!;
              let delta = chapterDelta(request.delta);
              if (request.baseRev < history.length) coverage.concurrentSubmission++;
              for (const revision of history.slice(request.baseRev)) {
                delta = chapterDelta(revision.delta).transform(delta, true);
              }
              const acknowledgement = { rev: history.length + 1, delta: parseDelta(delta) };
              trace.push({ step, id, commit: acknowledgement });
              server = server.compose(delta);
              history.push(acknowledgement);
              documents.push(server);
              client.request = null;
              client.acknowledgement = acknowledgement;
            });
          }
          const remoteHead = client.acknowledgement ? client.acknowledgement.rev - 1 : history.length;
          if (client.state.rev < remoteHead) {
            actions.push(() => {
              const end = client.state.rev + 1 + random(remoteHead - client.state.rev);
              const revisions = history.slice(client.state.rev, end);
              trace.push({ step, id, remote: revisions.map((revision) => revision.rev) });
              if (client.state.flight) coverage.remoteDuringFlight++;
              client.state = receiveRevisions(client.state, revisions);
            });
          }
          if (client.acknowledgement) {
            actions.push(() => {
              const acknowledgement = client.acknowledgement!;
              const end = acknowledgement.rev + random(history.length - acknowledgement.rev + 1);
              const revisions = history.slice(client.state.rev, end);
              trace.push({ step, id, ack: acknowledgement.rev, through: end });
              if (history.length > acknowledgement.rev) coverage.lateAck++;
              client.state = receiveRevisions(client.state, revisions, acknowledgement);
              client.acknowledgement = null;
            });
          }
        }
        if (actions.length === 0) throw new Error("No scheduler action available");
        actions[random(actions.length)]!();
        for (const client of clients) {
          expect(client.state.server).toEqual(documents[client.state.rev]!);
          if (client.state.flight) expect(sendChapter(client.state)).toBeNull();
          for (const delta of [syncDocument(client.state), client.state.flight, client.state.pending]) {
            expect(delta?.ops.every((op) => typeof op.insert !== "string" || op.insert.isWellFormed()) ?? true).toBe(true);
          }
        }
        step++;
      }
      expect(edits).toBe(editCount);
      for (const client of clients) {
        expect(client.edits).toBeGreaterThan(0);
        expect(syncDocument(client.state)).toEqual(server);
        expect(client.state.rev).toBe(history.length);
        expect(hasLocalChanges(client.state)).toBe(false);
      }
      for (const count of Object.values(coverage)) expect(count).toBeGreaterThan(0);
    } catch (error) {
      throw new Error(JSON.stringify({
        seed, step, edits, coverage, trace, failure: String(error),
        clients: clients.map((client) => ({ ...client, document: syncDocument(client.state) })),
        server,
      }), { cause: error });
    }
  });
}
