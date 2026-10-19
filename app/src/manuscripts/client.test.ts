import { describe, expect, test } from "bun:test";
import { createManuscriptClient, ManuscriptApiError } from "./client";
import { parseDelta, type Chapter, type CreateChapter } from "./contract";

const series = { id: 1, title: "Series", position: 0 };
const book = { id: 2, series_id: 1, title: "Book", position: 1 };
const chapter: Chapter = {
  id: 3, book_id: 2, title: "Chapter", position: 2, kind: "chapter",
  content: { ops: [{ insert: "Hello 🌍\n", attributes: { bold: true } }] }, head_rev: 0,
};
const scope = { seriesId: 1, bookId: 2, chapterId: 3 };
const metadata = { title: "New", position: 0 };
const createChapter: CreateChapter = { ...metadata, kind: "prologue" };
const update = { title: null, position: 3 };
const revision = { rev: 1, delta: { ops: [{ insert: "New" }] } };
const submission = { baseRev: 0, delta: revision.delta };
const root = "/series/1/books/2/chapters/3";
type Client = ReturnType<typeof createManuscriptClient>;

describe("manuscript request contracts", () => {
  const cases: Array<{
    name: string; call: (client: Client) => Promise<unknown>;
    path: string; method: string; status: number; response?: unknown; body?: unknown;
  }> = [
    { name: "list series", call: (c) => c.listSeries(), path: "/series", method: "GET", status: 200, response: [series] },
    { name: "create series", call: (c) => c.createSeries(metadata), path: "/series", method: "POST", status: 201, response: series, body: metadata },
    { name: "get series", call: (c) => c.getSeries(scope), path: "/series/1", method: "GET", status: 200, response: series },
    { name: "update series", call: (c) => c.updateSeries(scope, update), path: "/series/1", method: "PUT", status: 200, response: series, body: update },
    { name: "delete series", call: (c) => c.deleteSeries(scope), path: "/series/1", method: "DELETE", status: 204 },
    { name: "list books", call: (c) => c.listBooks(scope), path: "/series/1/books", method: "GET", status: 200, response: [book] },
    { name: "create book", call: (c) => c.createBook(scope, metadata), path: "/series/1/books", method: "POST", status: 201, response: book, body: metadata },
    { name: "get book", call: (c) => c.getBook(scope), path: "/series/1/books/2", method: "GET", status: 200, response: book },
    { name: "update book", call: (c) => c.updateBook(scope, update), path: "/series/1/books/2", method: "PUT", status: 200, response: book, body: update },
    { name: "delete book", call: (c) => c.deleteBook(scope), path: "/series/1/books/2", method: "DELETE", status: 204 },
    { name: "list chapters", call: (c) => c.listChapters(scope), path: "/series/1/books/2/chapters", method: "GET", status: 200, response: [chapter] },
    { name: "create chapter", call: (c) => c.createChapter(scope, createChapter), path: "/series/1/books/2/chapters", method: "POST", status: 201, response: chapter, body: createChapter },
    { name: "get chapter", call: (c) => c.getChapter(scope), path: root, method: "GET", status: 200, response: chapter },
    { name: "update chapter", call: (c) => c.updateChapter(scope, { kind: null }), path: root, method: "PUT", status: 200, response: chapter, body: { kind: null } },
    { name: "delete chapter", call: (c) => c.deleteChapter(scope), path: root, method: "DELETE", status: 204 },
    { name: "get deltas", call: (c) => c.getDeltas(scope, 0), path: `${root}/deltas?since=0`, method: "GET", status: 200, response: [revision] },
    { name: "post delta", call: (c) => c.postDelta(scope, submission), path: `${root}/deltas`, method: "POST", status: 200, response: revision, body: submission },
  ];
  for (const entry of cases) {
    test(entry.name, async () => {
      const client = createManuscriptClient({
        baseUrl: "http://api.test///",
        fetch: async (url, init) => {
          expect(url).toBe(`http://api.test${entry.path}`);
          expect(init?.method).toBe(entry.method);
          expect(init?.headers).toEqual({
            Accept: "application/json",
            ...(entry.body === undefined ? {} : { "Content-Type": "application/json" }),
          });
          expect(init?.body ?? null).toBe(entry.body === undefined ? null : JSON.stringify(entry.body));
          return entry.status === 204 ? new Response(null, { status: 204 })
            : Response.json(entry.response, { status: entry.status });
        },
      });
      expect(await entry.call(client)).toEqual(entry.response);
    });
  }
});

test("default API origin and empty history", async () => {
  const client = createManuscriptClient({ fetch: async (url) => {
    expect(url).toBe(`http://127.0.0.1:2908${root}/deltas?since=12`);
    return Response.json([]);
  } });
  expect(await client.getDeltas(scope, 12)).toEqual([]);
});

test("chapter creation preserves explicit content", async () => {
  const client = createManuscriptClient({ fetch: async (_, init) => {
    expect(JSON.parse(String(init?.body))).toEqual({ ...createChapter, content: chapter.content });
    return Response.json(chapter, { status: 201 });
  } });
  await client.createChapter(scope, { ...createChapter, content: chapter.content });
});

test("parses all supported Delta operations without dropping nested attributes or embeds", () => {
  const delta = { ops: [
    { insert: "" }, { insert: { image: "url" }, attributes: { nested: { x: [1, null] } } },
    { retain: { image: { alt: "new" } }, attributes: null }, { retain: 0 }, { delete: 2 },
  ] };
  expect(parseDelta(delta)).toEqual(delta);
});

test("rejects malformed Delta operations and unsafe total lengths", () => {
  for (const value of [null, {}, [], { ops: null }, { ops: [null] },
    { ops: [{}] }, { ops: [{ insert: "x", delete: 1 }] }, { ops: [{ insert: 1 }] },
    { ops: [{ retain: [] }] }, { ops: [{ delete: -1 }] }, { ops: [{ retain: 0.5 }] },
    { ops: [{ insert: "x", attributes: [] }] }, { ops: [{ insert: "x", extra: true }] },
    { ops: [{ retain: Number.MAX_SAFE_INTEGER }, { insert: "🌍" }] },
  ]) expect(() => parseDelta(value)).toThrow();
});

test("rejects malformed resource, collection, and revision responses at the boundary", async () => {
  const cases: Array<[unknown, (client: Client) => Promise<unknown>]> = [
    [null, (c) => c.getSeries(scope)],
    [{ ...series, id: Number.MAX_SAFE_INTEGER + 1 }, (c) => c.getSeries(scope)],
    [{ ...series, title: " " }, (c) => c.getSeries(scope)],
    [{ ...series, position: -1 }, (c) => c.getSeries(scope)],
    [{ ...series, position: 0.5 }, (c) => c.getSeries(scope)],
    [{ ...book, series_id: "1" }, (c) => c.getBook(scope)],
    [{ ...chapter, kind: "scene" }, (c) => c.getChapter(scope)],
    [{ ...chapter, book_id: null }, (c) => c.getChapter(scope)],
    [{ ...chapter, head_rev: undefined }, (c) => c.getChapter(scope)],
    [{ ...chapter, content: { ops: [{ insert: false }] } }, (c) => c.getChapter(scope)],
    [{}, (c) => c.listSeries()],
    [[series, {}], (c) => c.listSeries()],
    [[book, {}], (c) => c.listBooks(scope)],
    [[chapter, {}], (c) => c.listChapters(scope)],
    [[{ ...revision, rev: -1 }], (c) => c.getDeltas(scope, 0)],
    [{ ...revision, delta: {} }, (c) => c.postDelta(scope, submission)],
  ];
  for (const [response, call] of cases) {
    const client = createManuscriptClient({ fetch: async () => Response.json(response) });
    await expect(call(client)).rejects.toMatchObject({ kind: "invalid_response", status: 200, code: "INVALID_RESPONSE" });
  }
});

test("maps backend error messages, codes, and statuses", async () => {
  for (const [status, code] of [[404, "NOT_FOUND"], [409, "CONFLICT"], [422, "VALIDATION_ERROR"], [500, "SQLX"]] as const) {
    const client = createManuscriptClient({ fetch: async () => Response.json({ error: "backend message", code }, { status }) });
    try {
      await client.getChapter(scope);
      throw new Error("Expected rejection");
    } catch (error) {
      expect(error).toBeInstanceOf(ManuscriptApiError);
      expect(error).toMatchObject({ message: "backend message", status, code, kind: "http" });
    }
  }
});

test("rejects non-JSON success, malformed errors, and unexpected success statuses", async () => {
  for (const response of [
    new Response("not json"),
    new Response("proxy failure", { status: 502 }),
    Response.json({ error: "missing code" }, { status: 422 }),
    Response.json({ error: 1, code: "BAD" }, { status: 400 }),
    Response.json(series, { status: 202 }),
    new Response(null, { status: 204 }),
  ]) {
    const client = createManuscriptClient({ fetch: async () => response });
    await expect(client.getSeries(scope)).rejects.toMatchObject({ status: response.status, kind: "invalid_response" });
  }
});

test("keeps network failures distinct from server and parsing errors", async () => {
  const failure = new TypeError("Network unavailable");
  const client = createManuscriptClient({ fetch: async () => { throw failure; } });
  await expect(client.listSeries()).rejects.toBe(failure);
});
