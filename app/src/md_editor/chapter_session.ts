import Delta from "quill-delta";
import type { createManuscriptClient } from "../manuscripts/client";
import { parseDelta, type Chapter, type ChapterScope, type EditorScope, type PieceScope, type Revision } from "../manuscripts/contract";
import { sameScope } from "../manuscripts/story_tree";
import { chapterDelta, editChapter, hasLocalChanges, receiveRevisions, sendChapter, startChapterSync, syncDocument } from "./chapter_sync";
import { diffDocuments } from "./document_diff";

type Client = ReturnType<typeof createManuscriptClient>;
export type SyncProblem = { kind: "retrying" | "blocked"; message: string };

export type EditorDocument = Pick<Chapter, "title" | "content" | "head_rev">;

export class NoEditorChapter extends Error {}

async function loadPiece(client: Client, pieceId: number): Promise<{ scope: PieceScope; chapter: EditorDocument }> {
  const piece = await client.getPiece({ pieceId });
  if (piece.shape !== "leaf") throw new Error("Only a piece with text can open in the editor.");
  return {
    scope: { universeId: piece.universeId, pieceId },
    chapter: { title: piece.title, content: piece.content, head_rev: piece.headRev },
  };
}

export async function loadEditorDocument(client: Client, scope: EditorScope): Promise<EditorDocument> {
  return "pieceId" in scope ? (await loadPiece(client, scope.pieceId)).chapter : client.getChapter(scope);
}

export async function bootstrapEditorChapter(client: Client): Promise<ChapterScope> {
  const series = (await client.listSeries())[0] ?? await client.createSeries({ title: "My Series", position: 0 });
  const seriesScope = { seriesId: series.id };
  const book = (await client.listBooks(seriesScope))[0] ?? await client.createBook(seriesScope, { title: "My Book", position: 0 });
  const bookScope = { ...seriesScope, bookId: book.id };
  const chapter = (await client.listChapters(bookScope))[0] ?? await client.createChapter(bookScope, {
    title: "Chapter One", position: 0, kind: "chapter", content: { ops: [{ insert: "\n" }] },
  });
  return { ...bookScope, chapterId: chapter.id };
}

export async function loadEditorChapter(client: Client, search: string): Promise<{ scope: EditorScope; chapter: EditorDocument }> {
  const params = new URLSearchParams(search);
  if (params.has("pieceId")) {
    const pieceId = Number(params.get("pieceId"));
    if (!Number.isSafeInteger(pieceId) || pieceId <= 0) throw new Error("Piece URL requires a positive pieceId.");
    return loadPiece(client, pieceId);
  }
  const keys = ["seriesId", "bookId", "chapterId"];
  if (keys.some((key) => params.has(key))) {
    const [seriesId, bookId, chapterId] = keys.map((key) => Number(params.get(key)));
    if (![seriesId, bookId, chapterId].every((id) => Number.isSafeInteger(id) && id > 0)) {
      throw new Error("Chapter URL requires positive seriesId, bookId, and chapterId.");
    }
    const scope = { seriesId, bookId, chapterId };
    return { scope, chapter: await client.getChapter(scope) };
  }
  for (const series of await client.listSeries()) {
    for (const book of await client.listBooks({ seriesId: series.id })) {
      const scope = { seriesId: series.id, bookId: book.id };
      const chapter = (await client.listChapters(scope))[0];
      if (chapter) {
        const chapterScope = { ...scope, chapterId: chapter.id };
        return { scope: chapterScope, chapter: await client.getChapter(chapterScope) };
      }
    }
  }
  throw new NoEditorChapter("No chapter found. Create a chapter to start editing.");
}

export function createChapterSession(
  client: Pick<Client, "postDelta" | "getDeltas">,
  scope: EditorScope,
  chapter: Pick<EditorDocument, "content" | "head_rev">,
  publish: (document: Delta, dirty: boolean, problem: SyncProblem | null) => void,
) {
  let state = startChapterSync(chapter.content, chapter.head_rev);
  let running: Promise<void> | null = null;
  let aiReserved = false;
  let stopped = false;
  let problem: SyncProblem | null = null;
  let lastError: string | null = null;
  let acknowledgement: Revision | null = null;
  const notify = () => publish(syncDocument(state), hasLocalChanges(state), problem);

  function tick(): Promise<void> {
    if (running) return Promise.resolve();
    if (aiReserved || stopped || problem?.kind === "blocked") return Promise.resolve();
    running = synchronize().finally(() => { running = null; });
    return running;
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : "Unknown synchronization error";
  }

  function rememberFailure(error: unknown, uncertainPost: boolean) {
    const message = errorMessage(error);
    lastError = message;
    problem = uncertainPost || !(error instanceof TypeError)
      ? { kind: "blocked", message: `Save stopped: ${message}. Export local work before reload. Do not retry an uncertain submission.` }
      : { kind: "retrying", message: `Connection interrupted: ${message}. Local work retained; polling will retry.` };
    notify();
  }

  function flightMatches(revision: Revision): boolean {
    const flight = state.flight;
    if (flight === null) return false;
    const remote = chapterDelta(revision.delta);
    if (JSON.stringify(parseDelta(flight)) !== JSON.stringify(parseDelta(remote))) return false;
    return diffDocuments(state.server.compose(flight), state.server.compose(remote)).ops.length === 0;
  }

  function collapseFlight() {
    if (state.flight === null) return;
    state = { rev: state.rev, server: state.server, flight: null, pending: state.flight.compose(state.pending) };
  }

  async function recoverFlight() {
    const revisions = await client.getDeltas(scope, state.rev);
    if (stopped) return;
    let acknowledged = false;
    for (const revision of revisions) {
      if (!acknowledged && flightMatches(revision)) {
        state = receiveRevisions(state, [revision], revision);
        acknowledged = true;
      } else {
        state = receiveRevisions(state, [revision], null);
      }
    }
    if (stopped) return;
    if (acknowledged) {
      lastError = null;
      problem = null;
      notify();
      return;
    }
    collapseFlight();
    problem = {
      kind: "retrying",
      message: `Save stopped: ${lastError ?? "Unknown synchronization error"}. Local work retained; polling will retry.`,
    };
    notify();
  }

  async function synchronize() {
    let posting = false;
    try {
      if (acknowledgement === null && state.flight !== null) {
        await recoverFlight();
        return;
      }
      if (acknowledgement === null) {
        const submission = sendChapter(state);
        if (submission) {
          state = submission.state;
          posting = true;
          try {
            acknowledgement = await client.postDelta(scope, submission.request);
          } catch (error) {
            if (stopped) return;
            posting = false;
            lastError = errorMessage(error);
            problem = {
              kind: "retrying",
              message: `Save stopped: ${lastError}. Local work retained; polling will retry.`,
            };
            notify();
            return;
          }
          posting = false;
        }
      }
      if (stopped) return;
      const revisions = await client.getDeltas(scope, state.rev);
      if (stopped) return;
      state = receiveRevisions(state, revisions, acknowledgement);
      acknowledgement = null;
      lastError = null;
      problem = null;
      notify();
    } catch (error) {
      if (stopped) return;
      rememberFailure(error, posting);
    }
  }

  const clean = () => !stopped && problem?.kind !== "blocked" && !hasLocalChanges(state);

  return {
    document: () => syncDocument(state),
    canStartAi: () => !running && !aiReserved && clean(),
    reserveAi() {
      if (aiReserved || !clean()) return null;
      aiReserved = true;
      let released = false;
      return {
        async wait(signal: AbortSignal) {
          signal.throwIfAborted();
          let abort!: () => void;
          const cancelled = new Promise<never>((_resolve, reject) => {
            abort = () => reject(signal.reason);
            signal.addEventListener("abort", abort, { once: true });
          });
          try {
            await Promise.race([running, cancelled]);
            signal.throwIfAborted();
            if (released || !clean()) throw new Error("Wait for chapter saving and polling to finish before starting AI.");
          } finally {
            signal.removeEventListener("abort", abort);
          }
        },
        release() {
          if (released) return;
          released = true;
          aiReserved = false;
        },
      };
    },
    stop: () => { stopped = true; aiReserved = false; },
    edit(document: Delta) {
      if (stopped) return;
      state = editChapter(state, document);
      notify();
    },
    tick,
    async flush() {
      if (running) {
        await running;
        if (lastError) throw new Error(problem?.message ?? lastError);
      }
      if (stopped || problem?.kind === "blocked") throw new Error(problem?.message ?? "Chapter session stopped.");
      if (lastError && acknowledgement === null && state.flight !== null) throw new Error(problem?.message ?? lastError);
      while (hasLocalChanges(state)) {
        if (aiReserved) throw new Error("Cannot save chapter while AI has reserved the session.");
        await (running ?? tick());
        if (stopped || lastError) throw new Error(problem?.message ?? lastError ?? "Chapter session stopped.");
      }
    },
  };
}

export function canSwitchChapter(current: EditorScope | null, target: EditorScope, unavailable: boolean): boolean {
  return !unavailable && current !== null && !sameScope(current, target);
}
