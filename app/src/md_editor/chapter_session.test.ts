import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { createManuscriptClient } from "../manuscripts/client";
import { parseDelta, type Chapter, type Revision } from "../manuscripts/contract";
import { asSentence, bootstrapEditorChapter, canSwitchChapter, createChapterSession, loadEditorChapter } from "./chapter_session";
import { chapterDelta } from "./chapter_sync";

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
  expect(session.canStartAi()).toBe(true);
  expect(posts).toBe(1);
  expect(gets).toBe(0);
  session.edit(new Delta().insert("ABtext\n"));
  resolvePost(ack);
  await first;
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
  expect(gets).toBe(1);
});

test("failed POST waits for the next 1000 ms poll tick to recover before resubmitting", async () => {
  let posts = 0;
  let pollTime = 1000;
  const historyTimes: number[] = [];
  const stored: Revision[] = [];
  let error: string | null = null;
  const session = createChapterSession({
    postDelta: async (_scope, input) => {
      posts++;
      if (posts === 1) throw new Error("Save unavailable");
      const revision = { rev: stored.length + 1, delta: input.delta };
      stored.push(revision);
      return revision;
    },
    getDeltas: async (_scope, since) => {
      historyTimes.push(pollTime);
      return stored.filter((revision) => revision.rev > since);
    },
  }, scope, chapter, (_document, _dirty, problem) => { error = problem?.message ?? null; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  expect(posts).toBe(1);
  expect(historyTimes).toEqual([]);
  expect(error).toContain("Save stopped");
  expect(error).toContain("polling will retry");
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  session.edit(new Delta().insert("ABtext\n"));
  pollTime += 1000;
  await session.tick();
  expect(historyTimes).toEqual([2000]);
  expect(posts).toBe(1);
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
  pollTime += 1000;
  await session.tick();
  expect(posts).toBe(2);
  expect(stored).toEqual([{ rev: 1, delta: { ops: [{ insert: "AB" }] } }]);
  expect(error).toBeNull();
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
  expect(session.canStartAi()).toBe(true);
});

test("recovery network failure on the next tick still reports interruption and retains local work", async () => {
  let gets = 0;
  let error: unknown = null;
  let dirty = false;
  const session = createChapterSession({
    postDelta: async () => { throw new TypeError("POST offline"); },
    getDeltas: async () => {
      gets++;
      if (gets === 1) throw new TypeError("GET offline");
      return [ack];
    },
  }, scope, chapter, (_document, local, problem) => { dirty = local; error = problem; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  expect(gets).toBe(0);
  await session.tick();
  expect(gets).toBe(1);
  expect(error).toEqual({ kind: "retrying", message: "Connection interrupted: GET offline. Local work retained; polling will retry." });
  expect(dirty).toBe(true);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  await session.tick();
  expect(gets).toBe(2);
  expect(error).toBeNull();
  expect(dirty).toBe(false);
});

test("stopping after a failed POST before the next tick prevents recovery", async () => {
  let gets = 0;
  let publications = 0;
  const session = createChapterSession({
    postDelta: async () => { throw new TypeError("offline"); },
    getDeltas: async () => { gets++; return [ack]; },
  }, scope, chapter, () => { publications++; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  expect(gets).toBe(0);
  session.stop();
  const beforeTick = publications;
  await session.tick();
  expect(gets).toBe(0);
  expect(publications).toBe(beforeTick);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
});

test("lost POST response acknowledges the applied revision and does not resend", async () => {
  let posts = 0;
  let submitted: Revision["delta"] | null = null;
  let error: string | null = "unset";
  const session = createChapterSession({
    postDelta: async (_scope, input) => {
      posts++;
      submitted = input.delta;
      throw new TypeError("offline");
    },
    getDeltas: async () => submitted === null ? [] : [{ rev: 1, delta: submitted }],
  }, scope, chapter, (_document, _dirty, problem) => { error = problem?.message ?? null; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  await session.tick();
  expect(posts).toBe(1);
  expect(error).toBeNull();
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  expect(session.canStartAi()).toBe(true);
});

test("lost POST response acknowledges a server-transformed revision without resending", async () => {
  let posts = 0;
  let submitted: Revision["delta"] | null = null;
  const remote = new Delta().insert("R");
  const session = createChapterSession({
    postDelta: async (_scope, input) => {
      posts++;
      submitted = input.delta;
      throw new TypeError("offline");
    },
    getDeltas: async () => {
      if (submitted === null) return [];
      const stored = remote.transform(chapterDelta(submitted), true);
      return [{ rev: 1, delta: parseDelta(remote) }, { rev: 2, delta: parseDelta(stored) }];
    },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Ltext\n"));
  await session.tick();
  await session.tick();
  expect(posts).toBe(1);
  expect(session.document().ops).toEqual([{ insert: "RLtext\n" }]);
  expect(session.canStartAi()).toBe(true);
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
      "/series/1/books/2/chapters": [{ id: chapter.id, title: chapter.title, position: chapter.position, kind: chapter.kind, head_rev: chapter.head_rev }],
      "/series/1/books/2/chapters/3": chapter,
    };
    return Response.json(values[path]);
  } });
  expect((await loadEditorChapter(client, "")).scope).toEqual(scope);
  expect(paths).toHaveLength(5);
  expect(paths.at(-1)).toBe("/series/1/books/2/chapters/3");
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

test("bootstrap reuses the first chapter summary without needing content or creating resources", async () => {
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url, init) => {
    expect(init?.method).toBe("GET");
    if (url.endsWith("/series")) return Response.json([{ id: 1, title: "Series", position: 0 }]);
    if (url.endsWith("/books")) return Response.json([{ id: 2, series_id: 1, title: "Book", position: 0 }]);
    expect(url).toBe("http://api/series/1/books/2/chapters");
    return Response.json([{ id: 3, title: "Chapter", position: 0, kind: "chapter", head_rev: 0 }]);
  } });
  expect(await bootstrapEditorChapter(client)).toEqual(scope);
});

test("chapter switch requires a loaded idle session and a different full scope", () => {
  expect(canSwitchChapter(null, scope, false)).toBe(false);
  expect(canSwitchChapter(scope, scope, false)).toBe(false);
  expect(canSwitchChapter(scope, { ...scope, chapterId: 4 }, true)).toBe(false);
  expect(canSwitchChapter(scope, { ...scope, chapterId: 4 }, false)).toBe(true);
  expect(canSwitchChapter(scope, { ...scope, bookId: 4 }, false)).toBe(true);
});

test("flush awaits the active save and then saves edits queued during that request", async () => {
  let resolvePost!: (value: Revision) => void;
  const revisions: Revision[] = [];
  let posts = 0;
  const session = createChapterSession({
    postDelta: async (_scope, input) => {
      posts++;
      const revision = posts === 1
        ? await new Promise<Revision>((resolve) => { resolvePost = resolve; })
        : { rev: posts, delta: input.delta };
      revisions.push(revision);
      return revision;
    },
    getDeltas: async (_scope, since) => revisions.filter((revision) => revision.rev > since),
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  const polling = session.tick();
  session.edit(new Delta().insert("ABtext\n"));
  let flushed = false;
  const saving = session.flush().then(() => { flushed = true; });
  await Promise.resolve();
  expect(flushed).toBe(false);
  resolvePost(ack);
  await Promise.all([polling, saving]);
  expect(posts).toBe(2);
  expect(session.canStartAi()).toBe(true);
  expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
});

test("failed flush cannot bypass the next background recovery tick", async () => {
  let posts = 0;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; throw new TypeError("offline"); },
    getDeltas: async () => { gets++; return []; },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  await expect(session.flush()).rejects.toThrow("offline");
  expect(posts).toBe(1);
  await expect(session.flush()).rejects.toThrow("offline");
  expect(posts).toBe(1);
  expect(gets).toBe(0);
  await session.tick();
  expect(gets).toBe(1);
  expect(posts).toBe(1);
  await expect(session.flush()).rejects.toThrow("offline");
  expect(posts).toBe(2);
  expect(gets).toBe(1);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
});

test("flush awaiting a failed active POST does not start recovery", async () => {
  let rejectPost!: (error: Error) => void;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => new Promise<Revision>((_resolve, reject) => { rejectPost = reject; }),
    getDeltas: async () => { gets++; return [ack]; },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  const posting = session.tick();
  const flushing = session.flush();
  rejectPost(new TypeError("offline"));
  await expect(flushing).rejects.toThrow("offline");
  await posting;
  expect(gets).toBe(0);
  await session.tick();
  expect(gets).toBe(1);
  expect(session.canStartAi()).toBe(true);
});

test("flush rejects failed acknowledgement history and retries only history on the next attempt", async () => {
  let posts = 0;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; return ack; },
    getDeltas: async () => { if (++gets === 1) throw new TypeError("offline"); return [ack]; },
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  await expect(session.flush()).rejects.toThrow("offline");
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  await session.flush();
  expect(posts).toBe(1);
  expect(gets).toBe(2);
  expect(session.canStartAi()).toBe(true);
});

test("flush rejects dirty work in a reserved session and saves after release", async () => {
  let posts = 0;
  let gets = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; return ack; },
    getDeltas: async () => { gets++; return [ack]; },
  }, scope, chapter, () => {});
  const reservation = session.reserveAi();
  expect(reservation).not.toBeNull();
  session.edit(new Delta().insert("Atext\n"));
  await expect(session.flush()).rejects.toThrow("AI has reserved the session");
  expect(posts).toBe(0);
  expect(gets).toBe(0);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  reservation!.release();
  await session.flush();
  expect(posts).toBe(1);
  expect(gets).toBe(1);
  expect(session.canStartAi()).toBe(true);
});

test("chapter switch boundary flush succeeds and loads the selected target after recovery", async () => {
  let posts = 0;
  let gets = 0;
  let dirty = false;
  const session = createChapterSession({
    postDelta: async () => { posts++; throw new TypeError("offline"); },
    getDeltas: async () => { gets++; return [ack]; },
  }, scope, chapter, (_document, local) => { dirty = local; });
  const target = { ...scope, chapterId: 4 };
  const urls: string[] = [];
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url) => {
    urls.push(url);
    return Response.json({ ...chapter, id: target.chapterId });
  } });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  await expect(session.flush()).rejects.toThrow("offline");
  expect(dirty).toBe(true);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  expect(posts).toBe(1);
  expect(gets).toBe(0);
  expect(urls).toEqual([]);
  await session.tick();
  expect(dirty).toBe(false);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
  await session.flush();
  expect(posts).toBe(1);
  expect(gets).toBe(1);
  expect(canSwitchChapter(scope, target, false)).toBe(true);
  const loaded = await loadEditorChapter(client, "?seriesId=1&bookId=2&chapterId=4");
  expect(loaded.scope).toEqual(target);
  expect(loaded.chapter).toMatchObject({ id: target.chapterId });
  expect(urls).toEqual(["http://api/series/1/books/2/chapters/4"]);
});

for (const alreadyRunning of [false, true]) {
  test(`AI reservation saves pending work and typing during an ${alreadyRunning ? "active" : "idle"} save`, async () => {
    let resolvePost!: (value: Revision) => void;
    const revisions: Revision[] = [];
    const session = createChapterSession({
      postDelta: async (_scope, request) => {
        const revision = { rev: revisions.length + 1, delta: request.delta };
        revisions.push(revision);
        if (revision.rev === 1) return new Promise((resolve) => { resolvePost = resolve; });
        return revision;
      },
      getDeltas: async (_scope, since) => revisions.filter((revision) => revision.rev > since),
    }, scope, chapter, () => {});
    session.edit(new Delta().insert("Atext\n"));
    if (alreadyRunning) void session.tick();
    const reservation = session.reserveAi()!;
    const waiting = reservation.wait(new AbortController().signal);
    session.edit(new Delta().insert("ABtext\n"));
    resolvePost(revisions[0]!);
    await waiting;
    expect(revisions).toHaveLength(2);
    expect(session.document().ops).toEqual([{ insert: "ABtext\n" }]);
    reservation.release();
  });
}

test("AI reservation aborts while a save continues", async () => {
  let resolvePost!: (value: Revision) => void;
  let dirty = false;
  const session = createChapterSession({
    postDelta: async () => new Promise((resolve) => { resolvePost = resolve; }),
    getDeltas: async () => [ack],
  }, scope, chapter, (_document, local) => { dirty = local; });
  session.edit(new Delta().insert("Atext\n"));
  const saving = session.tick();
  const reservation = session.reserveAi()!;
  const controller = new AbortController();
  const waiting = reservation.wait(controller.signal);
  controller.abort();
  await expect(waiting).rejects.toThrow();
  reservation.release();
  resolvePost(ack);
  await saving;
  expect(dirty).toBe(false);
  expect(session.document().ops).toEqual([{ insert: "Atext\n" }]);
});

test("AI reservation reports a failed save without retrying it", async () => {
  let posts = 0;
  const session = createChapterSession({
    postDelta: async () => { posts++; throw new TypeError("offline"); },
    getDeltas: async () => [ack],
  }, scope, chapter, () => {});
  session.edit(new Delta().insert("Atext\n"));
  const reservation = session.reserveAi()!;
  await expect(reservation.wait(new AbortController().signal)).rejects.toThrow("Save stopped: offline");
  expect(posts).toBe(1);
  reservation.release();
});

test("AI reservation reports a blocked active poll", async () => {
  let rejectPoll!: (error: Error) => void;
  const session = createChapterSession({
    postDelta: async () => ack,
    getDeltas: async () => new Promise((_resolve, reject) => { rejectPoll = reject; }),
  }, scope, chapter, () => {});
  const polling = session.tick();
  const reservation = session.reserveAi()!;
  const waiting = reservation.wait(new AbortController().signal);
  rejectPoll(new Error("bad history"));
  await polling;
  await expect(waiting).rejects.toThrow("Save stopped: bad history");
  reservation.release();
  expect(session.reserveAi()).toBeNull();
});

test("piece URL loads the leaf in its universe without a book series", async () => {
  const urls: string[] = [];
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async (url) => {
    urls.push(url);
    if (url.endsWith("/pieces/9")) return Response.json({
      id: 9, universeId: 5, containerId: null, parentId: null, title: "Loose", position: 0,
      label: "short_story", format: "prose", shape: "leaf", content: { ops: [{ insert: "Loose\n" }] }, headRev: 3,
    });
    throw new Error(`Unexpected ${url}`);
  } });
  const loaded = await loadEditorChapter(client, "?seriesId=1&bookId=2&chapterId=3&pieceId=9");
  expect(loaded).toEqual({
    scope: { universeId: 5, pieceId: 9 },
    chapter: { title: "Loose", content: { ops: [{ insert: "Loose\n" }] }, head_rev: 3 },
  });
  expect(urls).toEqual(["http://api/pieces/9"]);
  expect(canSwitchChapter(loaded.scope, { seriesId: 4, bookId: 2, chapterId: 3 }, false)).toBe(true);
  expect(canSwitchChapter(loaded.scope, { universeId: 5, pieceId: 9 }, false)).toBe(false);
});

test("piece URL rejects groups and invalid ids", async () => {
  const client = createManuscriptClient({ baseUrl: "http://api", fetch: async () => Response.json({
    id: 9, universeId: 5, containerId: null, parentId: null, title: "Play", position: 0,
    label: "screenplay", format: "script", shape: "group", children: [],
  }) });
  await expect(loadEditorChapter(client, "?pieceId=9")).rejects.toThrow("Only a piece with text");
  await expect(loadEditorChapter(client, "?pieceId=0")).rejects.toThrow("positive pieceId");
});

test("asSentence ends the message with exactly one sentence mark", () => {
  expect(asSentence("Something went wrong. Try again.")).toBe("Something went wrong. Try again.");
  expect(asSentence("Failed to fetch")).toBe("Failed to fetch.");
  expect(asSentence(" Wait… ")).toBe("Wait…");
});

test("composed save retry status has no double period", async () => {
  let error: unknown = null;
  const session = createChapterSession({
    postDelta: async () => { throw new Error("Something went wrong. Try again."); },
    getDeltas: async () => [],
  }, scope, chapter, (_document, _local, problem) => { error = problem; });
  session.edit(new Delta().insert("Atext\n"));
  await session.tick();
  expect(error).toEqual({ kind: "retrying", message: "Save stopped: Something went wrong. Try again. Local work retained; polling will retry." });
});
