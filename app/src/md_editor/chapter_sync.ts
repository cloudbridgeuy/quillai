import Delta from "quill-delta";
import { parseDelta, type ManuscriptDelta, type Revision, type SubmitDelta } from "../manuscripts/contract";

export type ChapterSync = {
  rev: number;
  server: Delta;
  flight: Delta | null;
  pending: Delta;
};

export function chapterDelta(document: ManuscriptDelta): Delta {
  return new Delta(document.ops.map(({ attributes, ...op }) =>
    attributes == null ? op : { ...op, attributes }));
}

export function startChapterSync(content: ManuscriptDelta, rev: number): ChapterSync {
  return { rev, server: chapterDelta(content), flight: null, pending: new Delta() };
}

export function syncDocument(state: ChapterSync): Delta {
  return state.server.compose(state.flight ?? new Delta()).compose(state.pending);
}

export function hasLocalChanges(state: ChapterSync): boolean {
  return state.flight !== null || state.pending.ops.length > 0;
}

export function editChapter(state: ChapterSync, document: Delta): ChapterSync {
  const baseline = state.server.compose(state.flight ?? new Delta());
  const pending = baseline.diff(document).ops.length === 0
    ? new Delta()
    : state.pending.compose(syncDocument(state).diff(document));
  return { ...state, pending };
}

export function sendChapter(state: ChapterSync): { state: ChapterSync; request: SubmitDelta } | null {
  if (state.flight !== null || state.pending.ops.length === 0) return null;
  return {
    state: { ...state, flight: state.pending, pending: new Delta() },
    request: { baseRev: state.rev, delta: parseDelta(state.pending) },
  };
}

export function receiveRevisions(state: ChapterSync, revisions: Revision[], acknowledgement: Revision | null = null): ChapterSync {
  let next = state;
  for (const revision of revisions) {
    if (revision.rev <= next.rev) continue;
    if (revision.rev !== next.rev + 1) throw new Error("Missing chapter revision; local work retained.");
    const remote = chapterDelta(revision.delta);
    const server = next.server.compose(remote);
    if (revision.rev === acknowledgement?.rev) {
      if (next.flight === null || JSON.stringify(parseDelta(remote)) !== JSON.stringify(parseDelta(acknowledgement.delta)) ||
        next.server.compose(next.flight).diff(server).ops.length !== 0) {
        throw new Error("Chapter acknowledgement mismatch; local work retained.");
      }
      next = { ...next, server, rev: revision.rev, flight: null };
      continue;
    }
    const afterFlight = next.flight ? next.flight.transform(remote, false) : remote;
    next = {
      server,
      rev: revision.rev,
      flight: next.flight ? remote.transform(next.flight, true) : null,
      pending: afterFlight.transform(next.pending, true),
    };
  }
  if (acknowledgement && next.rev < acknowledgement.rev) {
    throw new Error("Chapter acknowledgement not in history; local work retained.");
  }
  return next;
}
