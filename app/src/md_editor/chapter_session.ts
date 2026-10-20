import Delta from "quill-delta";
import type { createManuscriptClient } from "../manuscripts/client";
import type { Chapter, ChapterScope, Revision } from "../manuscripts/contract";
import { editChapter, hasLocalChanges, receiveRevisions, sendChapter, startChapterSync, syncDocument } from "./chapter_sync";

type Client = ReturnType<typeof createManuscriptClient>;
export type SyncProblem = { kind: "retrying" | "blocked"; message: string };

export class NoEditorChapter extends Error {}

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

export async function loadEditorChapter(client: Client, search: string): Promise<{ scope: ChapterScope; chapter: Chapter }> {
  const params = new URLSearchParams(search);
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
  scope: ChapterScope,
  chapter: Chapter,
  publish: (document: Delta, dirty: boolean, problem: SyncProblem | null) => void,
) {
  let state = startChapterSync(chapter.content, chapter.head_rev);
  let running: Promise<void> | null = null;
  let stopped = false;
  let failure: string | null = null;
  let lastError: string | null = null;
  let acknowledgement: Revision | null = null;
  const notify = () => publish(syncDocument(state), hasLocalChanges(state), failure ? { kind: "blocked", message: failure } : null);

  function tick(): Promise<void> {
    if (running) return Promise.resolve();
    if (stopped || failure) return Promise.resolve();
    running = synchronize().finally(() => { running = null; });
    return running;
  }

  async function synchronize() {
    let posting = false;
    try {
      if (acknowledgement === null) {
        const submission = sendChapter(state);
        if (submission) {
          state = submission.state;
          posting = true;
          acknowledgement = await client.postDelta(scope, submission.request);
          posting = false;
        }
      }
      if (stopped) return;
      const revisions = await client.getDeltas(scope, state.rev);
      if (stopped) return;
      state = receiveRevisions(state, revisions, acknowledgement);
      acknowledgement = null;
      lastError = null;
      notify();
    } catch (error) {
      if (stopped) return;
      const message = error instanceof Error ? error.message : "Unknown synchronization error";
      lastError = message;
      if (posting || !(error instanceof TypeError)) {
        failure = `Save stopped: ${message}. Export local work before reload. Do not retry an uncertain submission.`;
      }
      publish(syncDocument(state), hasLocalChanges(state), failure
        ? { kind: "blocked", message: failure }
        : { kind: "retrying", message: `Connection interrupted: ${message}. Local work retained; polling will retry.` });
    }
  }

  return {
    document: () => syncDocument(state),
    canStartAi: () => !running && !stopped && !failure && !hasLocalChanges(state),
    stop: () => { stopped = true; },
    edit(document: Delta) {
      if (stopped) return;
      state = editChapter(state, document);
      notify();
    },
    tick,
    async flush() {
      if (running) {
        await running;
        if (lastError) throw new Error(failure ?? lastError);
      }
      if (stopped || failure) throw new Error(failure ?? "Chapter session stopped.");
      while (hasLocalChanges(state)) {
        await tick();
        if (stopped || lastError) throw new Error(failure ?? lastError ?? "Chapter session stopped.");
      }
    },
  };
}

export function canSwitchChapter(current: ChapterScope | null, target: ChapterScope, unavailable: boolean): boolean {
  return !unavailable && current !== null &&
    (current.seriesId !== target.seriesId || current.bookId !== target.bookId || current.chapterId !== target.chapterId);
}
