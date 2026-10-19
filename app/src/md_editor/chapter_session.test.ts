import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { createManuscriptClient } from "../manuscripts/client";
import { parseDelta, type Chapter, type Revision } from "../manuscripts/contract";
import { bootstrapEditorChapter, createChapterSession, loadEditorChapter } from "./chapter_session";

const scope = { seriesId: 1, bookId: 2, chapterId: 3 };
const chapter: Chapter = { id: 3, book_id: 2, title: "Chapter", position: 0, kind: "chapter", content: { ops: [{ insert: "text\n" }] }, head_rev: 0 };
const ack: Revision = { rev: 1, delta: { ops: [{ insert: "A" }] } };

test("serializes polling and posting while retaining edits made during the request", async () => {
  let resolvePost!: (value: Revision) => void;
  let posts = 0;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; return new Promise((resolve) => { resolvePost = resolve; }); },
    getDeltas: async () => { gets++; return [ack]; },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  const first = session.tick();
  await session.tick();
  expect(session.canStartAi()).toBe(false);
  expect(posts).toBe(1);
  expect(gets).toBe(0);
  session.edit(new Delta().insert("ABtext\n"));
  resolvePost(ack);
  await first;
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
  expect(gets).toBe(1);
});

test("lost POST response blocks retry and retains flight plus new pending for export", async () => {
  let posts = 0;
  let error: string | null = null;
  const session = createChapterSession({
    postDelta: async () => { posts++; throw new TypeError("offline"); },
    getDeltas: async () => [],
  }, scope, chapter, (_document, _dirty, problem) => { error = problem?.message ?? null; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  await session.tick();
  expect(posts).toBe(1);
  expect(error).toContain("Do not retry");
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  expect(session.canStartAi()).toBe(false);
});

test("GET network failure after POST retries only history, not the submission", async () => {
  let posts = 0;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; return ack; },
    getDeltas: async () => { gets++; if (gets === 1) throw new TypeError("offline"); return [ack]; },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  session.edit(new Delta().insert("ABtext\n"));
  await session.tick();
  expect(posts).toBe(1);
  expect(gets).toBe(2);
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
});

test("poll arriving after a new local edit transforms pending and next POST uses new base", async () => {
  let resolveGet!: (value: Revision[]) => void;
  const requests: unknown[] = [];
  let gets = 0;
  const remote: Revision = { rev: 1, delta: { ops: [{ insert: "R" }] } };
  const session = createChapterSession({
    postDelta: async (_scope, input) => {
      requests.push(input);
      return { rev: 2, delta: input.delta };
    },
    getDeltas: async () => {
      gets++;
      if (gets === 1) return new Promise((resolve) => { resolveGet = resolve; });
      return [{ rev: 2, delta: parseDelta(new Delta().retain(1).insert("L")) }];
    },
  }, scope, chapter, () => {});
  const polling = session.tick();
  session.edit(new Delta().insert("Ltext\n"));
  resolveGet([remote]);
  await polling;
  await session.tick();
  expect(requests).toEqual([{ baseRev: 1, delta: { ops: [{ retain: 1 }, { insert: "L" }] } }]);
  expect(session.document().ops).toEqual([{ insert: "RLtext\n" }]);
  expect(session.canStartAi()).toBe(true);
});

test("unavailable history blocks instead of replacing unsaved work", async () => {
  let dirty = false;
  let error: string | null = null;
  const session = createChapterSession({
    postDelta: async () => ack,
    getDeltas: async () => { throw new Error("409 history unavailable"); },
  }, scope, chapter, (_document, local, problem) => { dirty = local; error = problem?.message ?? null; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  expect(dirty).toBe(true);
  expect(error).toContain("Save stopped");
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
});

test("stopping an unmounted session prevents late history publication", async () => {
  let resolveGet!: (value: Revision[]) => void;
  let publications = 0;
  const session = createChapterSession({
    postDelta: async () => ack,
    getDeltas: async () => new Promise((resolve) => { resolveGet = resolve; }),
  }, scope, chapter, () => { publications++; });
  const polling = session.tick();
  session.stop();
  resolveGet([ack]);
  await polling;
  expect(publications).toBe(0);
});

test("reload fetches selected chapter from server at current head", async () => {
  const urls: string[] = [];
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url) => {
    urls.push(url);
    return Response.json({ ...chapter, content: { ops: [{ insert: "saved\n" }] }, head_rev: 19 });
  } });
  const loaded = await loadEditorChapter(client, "?seriesId=1&bookId=2&chapterId=3");
  expect(loaded.chapter.head_rev).toBe(19);
  expect(loaded.chapter.content.ops).toEqual([{ insert: "saved\n" }]);
  expect(urls).toEqual(["http://api/series/1/books/2/chapters/3"]);
});

test("bootstrap walks real hierarchy, skips empty books, and never creates resources", async () => {
  const paths: string[] = [];
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url, init) => {
    expect(init?.method).toBe("GET");
    const path = url.replace("http://api", "");
    paths.push(path);
    const values: Record<string, unknown> = {
      "/series": [{ id: 1, title: "Series", position: 0 }],
      "/series/1/books": [{ id: 8, series_id: 1, title: "Empty", position: 0 }, { id: 2, series_id: 1, title: "Book", position: 1 }],
      "/series/1/books/8/chapters": [],
      "/series/1/books/2/chapters": [chapter],
    };
    return Response.json(values[path]);
  } });
  expect((await loadEditorChapter(client, "")).scope).toEqual(scope);
  expect(paths).toHaveLength(4);
});

test("invalid explicit scope and empty hierarchy report errors without guessing IDs", async () => {
  let calls = 0;
  const client = createManuscriptClient({ fetch: async () => { calls++; return Response.json([]); } });
  await expect(loadEditorChapter(client, "?chapterId=3")).rejects.toThrow("requires positive");
  expect(calls).toBe(0);
  await expect(loadEditorChapter(client, "")).rejects.toThrow("No chapter found");
});

test("explicit bootstrap creates missing hierarchy and a valid newline document", async () => {
  const created: string[] = [];
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url, init) => {
    if (init?.method === "GET") return Response.json([]);
    created.push(url);
    const input = JSON.parse(String(init?.body));
    if (url.endsWith("/series")) return Response.json({ ...input, id: 1 }, { status: 201 });
    if (url.endsWith("/books")) return Response.json({ ...input, id: 2, series_id: 1 }, { status: 201 });
    expect(input.content).toEqual({ ops: [{ insert: "\n" }] });
    return Response.json({ ...input, id: 3, book_id: 2, head_rev: 0 }, { status: 201 });
  } });
  expect(await bootstrapEditorChapter(client)).toEqual(scope);
  expect(created).toEqual(["http://api/series", "http://api/series/1/books", "http://api/series/1/books/2/chapters"]);
});
