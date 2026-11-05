import Delta from "quill-delta";
import { ManuscriptApiError, type createManuscriptClient } from "../manuscripts/client";
import { parseDelta, type Chapter, type ChapterScope, type EditorScope, type PieceScope, type Revision } from "../manuscripts/contract";
import { sameScope } from "../manuscripts/story_tree";
import { chapterDelta, editChapter, hasLocalChanges, receiveRevisions, sendChapter, startChapterSync, syncDocument } from "./chapter_sync";
import { diffDocuments } from "./document_diff";
import { defaultLocale, formatMessage } from "../i18n/messages";

type Client = ReturnType<typeof createManuscriptClient>;
export type SyncProblem = { kind: "retrying" | "blocked"; message: string; detail?: string };

export type EditorDocument = Pick<Chapter, "title" | "content" | "head_rev">;

export class NoEditorChapter extends Error {}

async function loadPiece(client: Client, pieceId: number): Promise<{ scope: PieceScope; chapter: EditorDocument }> {
  const piece = await client.getPiece({ pieceId });
  if (piece.shape !== "leaf") throw new Error(formatMessage(defaultLocale, "sync.onlyTextPiece"));
  return {
    scope: { universeId: piece.universeId, pieceId },
    chapter: { title: piece.title, content: piece.content, head_rev: piece.headRev },
  };
}

export async function loadEditorDocument(client: Client, scope: EditorScope): Promise<EditorDocument> {
  return "pieceId" in scope ? (await loadPiece(client, scope.pieceId)).chapter : client.getChapter(scope);
}

export async function bootstrapEditorChapter(client: Client): Promise<ChapterScope> {
  const series = (await client.listSeries())[0] ?? await client.createSeries({ title: formatMessage(defaultLocale, "sync.defaultSeries"), position: 0 });
  const seriesScope = { seriesId: series.id };
  const book = (await client.listBooks(seriesScope))[0] ?? await client.createBook(seriesScope, { title: formatMessage(defaultLocale, "sync.defaultBook"), position: 0 });
  const bookScope = { ...seriesScope, bookId: book.id };
  const chapter = (await client.listChapters(bookScope))[0] ?? await client.createChapter(bookScope, {
    title: formatMessage(defaultLocale, "sync.firstChapter"), position: 0, kind: "chapter", content: { ops: [{ insert: "\n" }] },
  });
  return { ...bookScope, chapterId: chapter.id };
}

export async function loadEditorChapter(client: Client, search: string): Promise<{ scope: EditorScope; chapter: EditorDocument }> {
  const params = new URLSearchParams(search);
  if (params.has("pieceId")) {
    const pieceId = Number(params.get("pieceId"));
    if (!Number.isSafeInteger(pieceId) || pieceId <= 0) throw new Error(formatMessage(defaultLocale, "sync.pieceUrlInvalid"));
    return loadPiece(client, pieceId);
  }
  const keys = ["seriesId", "bookId", "chapterId"];
  if (keys.some((key) => params.has(key))) {
    const [seriesId, bookId, chapterId] = keys.map((key) => Number(params.get(key)));
    if (![seriesId, bookId, chapterId].every((id) => Number.isSafeInteger(id) && id > 0)) {
      throw new Error(formatMessage(defaultLocale, "sync.chapterUrlInvalid"));
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
  throw new NoEditorChapter(formatMessage(defaultLocale, "sync.noChapter"));
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
  let lastDetail: string | undefined;
  let acknowledgement: Revision | null = null;
  const notify = () => publish(syncDocument(state), hasLocalChanges(state), problem);

  function tick(): Promise<void> {
    if (running) return Promise.resolve();
    if (aiReserved || stopped || problem?.kind === "blocked") return Promise.resolve();
    return runSync();
  }

  function runSync(): Promise<void> {
    running = synchronize().finally(() => { running = null; });
    return running;
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : formatMessage(defaultLocale, "sync.unknownSynchronization");
  }

  function rememberFailure(error: unknown, uncertainPost: boolean) {
    const message = errorMessage(error);
    lastError = message;
    lastDetail = error instanceof ManuscriptApiError && error.detail !== error.message ? error.detail : undefined;
    problem = uncertainPost || !(error instanceof TypeError)
      ? { kind: "blocked", message: formatMessage(defaultLocale, "sync.saveStopped", { message }) }
      : { kind: "retrying", message: formatMessage(defaultLocale, "sync.connectionInterrupted", { message }) };
    if (lastDetail) problem.detail = lastDetail;
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
      lastDetail = undefined;
      problem = null;
      notify();
      return;
    }
    collapseFlight();
    problem = {
      kind: "retrying",
      message: formatMessage(defaultLocale, "sync.saveRetry", { message: lastError ?? formatMessage(defaultLocale, "sync.unknownSynchronization") }),
      ...(lastDetail ? { detail: lastDetail } : {}),
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
            lastDetail = error instanceof ManuscriptApiError && error.detail !== error.message ? error.detail : undefined;
            problem = {
              kind: "retrying",
              message: formatMessage(defaultLocale, "sync.saveRetry", { message: lastError }),
              ...(lastDetail ? { detail: lastDetail } : {}),
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
      lastDetail = undefined;
      problem = null;
      notify();
    } catch (error) {
      if (stopped) return;
      rememberFailure(error, posting);
    }
  }

  return {
    revision: () => state.rev,
    hasPendingWork: () => hasLocalChanges(state) || acknowledgement !== null || problem?.kind === "retrying",
    isBusy: () => running !== null,
    isReserved: () => aiReserved,
    document: () => syncDocument(state),
    canStartAi: () => !stopped && !aiReserved && problem?.kind !== "blocked",
    reserveAi() {
      if (aiReserved || stopped || problem?.kind === "blocked") return null;
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
            if (running) await Promise.race([running, cancelled]);
            while (true) {
              signal.throwIfAborted();
              if (released || stopped || lastError || problem?.kind === "blocked") {
                throw new Error(problem?.message ?? lastError ?? formatMessage(defaultLocale, "sync.sessionStopped"));
              }
              if (!hasLocalChanges(state)) break;
              await Promise.race([runSync(), cancelled]);
            }
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
      if (stopped || problem?.kind === "blocked") throw new Error(problem?.message ?? formatMessage(defaultLocale, "sync.sessionStopped"));
      if (lastError && acknowledgement === null && state.flight !== null) throw new Error(problem?.message ?? lastError);
      while (hasLocalChanges(state)) {
        if (aiReserved) throw new Error(formatMessage(defaultLocale, "sync.aiReserved"));
        await (running ?? tick());
        if (stopped || lastError) throw new Error(problem?.message ?? lastError ?? formatMessage(defaultLocale, "sync.sessionStopped"));
      }
    },
  };
}

export function canSwitchChapter(current: EditorScope | null, target: EditorScope, unavailable: boolean): boolean {
  return !unavailable && current !== null && !sameScope(current, target);
}
