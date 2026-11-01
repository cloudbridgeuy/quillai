import { describe, expect, test } from "bun:test";
import { createManuscriptClient, ManuscriptApiError } from "./client";
import { parseDelta, type Chapter, type CreateChapter } from "./contract";

const series = { id: 1, title: "Series", position: 0 };
const book = { id: 2, series_id: 1, title: "Book", position: 1 };
const chapter: Chapter = {
  id: 3, book_id: 2, title: "Chapter", position: 2, kind: "chapter",
  content: { ops: [{ insert: "Hello 🌍\n", attributes: { bold: true } }] }, head_rev: 0,
};
const { content: _content, book_id: _bookId, ...chapterSummary } = chapter;
const scope = { seriesId: 1, bookId: 2, chapterId: 3 };
const metadata = { title: "New", position: 0 };
const createChapter: CreateChapter = { ...metadata, kind: "prologue" };
const update = { title: null, position: 3 };
const revision = { rev: 1, delta: { ops: [{ insert: "New" }] } };
const submission = { baseRev: 0, delta: revision.delta };
const root = "/series/1/books/2/chapters/3";
const container = { id: 1, universeId: 5, title: "Series", position: 0, kind: "book_series" };
const piece = { id: 9, universeId: 5, containerId: null, parentId: null, title: "Loose", position: 0, label: "short_story", format: "prose", shape: "leaf" };
const pieceDetail = { ...piece, content: { ops: [{ insert: "Loose\n" }] }, headRev: 2 };
const group = { ...piece, id: 10, shape: "group", label: "screenplay", format: "script", children: [{ ...piece, id: 11, parentId: 10, label: "scene", format: "script" }] };
const newPiece = { ...metadata, label: "short_story", shape: "leaf", format: "prose" } as const;
const pieceScope = { seriesId: 1, universeId: 5, pieceId: 9 };
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
    { name: "list chapters", call: (c) => c.listChapters(scope), path: "/series/1/books/2/chapters", method: "GET", status: 200, response: [chapterSummary] },
    { name: "create chapter", call: (c) => c.createChapter(scope, createChapter), path: "/series/1/books/2/chapters", method: "POST", status: 201, response: chapter, body: createChapter },
    { name: "get chapter", call: (c) => c.getChapter(scope), path: root, method: "GET", status: 200, response: chapter },
    { name: "update chapter", call: (c) => c.updateChapter(scope, { kind: null }), path: root, method: "PUT", status: 200, response: chapter, body: { kind: null } },
    { name: "delete chapter", call: (c) => c.deleteChapter(scope), path: root, method: "DELETE", status: 204 },
    { name: "get deltas", call: (c) => c.getDeltas(scope, 0), path: `${root}/deltas?since=0`, method: "GET", status: 200, response: [revision] },
    { name: "post delta", call: (c) => c.postDelta(scope, submission), path: `${root}/deltas`, method: "POST", status: 200, response: revision, body: submission },
    { name: "list universes", call: (c) => c.listUniverses(), path: "/universes", method: "GET", status: 200, response: [series] },
    { name: "create universe", call: (c) => c.createUniverse(metadata), path: "/universes", method: "POST", status: 201, response: series, body: metadata },
    { name: "delete universe", call: (c) => c.deleteUniverse({ universeId: 5 }), path: "/universes/5", method: "DELETE", status: 204 },
    { name: "restore universe", call: (c) => c.restoreUniverse({ universeId: 5 }), path: "/universes/5/restore", method: "POST", status: 200, response: series },
    { name: "create container", call: (c) => c.createContainer({ universeId: 5 }, { ...metadata, kind: "collection" }), path: "/universes/5/containers", method: "POST", status: 201, response: container, body: { ...metadata, kind: "collection" } },
    { name: "delete container", call: (c) => c.deleteContainer({ containerId: 1 }), path: "/containers/1", method: "DELETE", status: 204 },
    { name: "restore container", call: (c) => c.restoreContainer({ containerId: 1 }), path: "/containers/1/restore", method: "POST", status: 200, response: container },
    { name: "create loose piece", call: (c) => c.createLoosePiece({ universeId: 5 }, newPiece), path: "/universes/5/pieces", method: "POST", status: 201, response: piece, body: newPiece },
    { name: "create container piece", call: (c) => c.createContainerPiece({ containerId: 1 }, newPiece), path: "/containers/1/pieces", method: "POST", status: 201, response: piece, body: newPiece },
    { name: "create child", call: (c) => c.createChild({ pieceId: 10 }, { ...metadata, label: "scene" }), path: "/pieces/10/children", method: "POST", status: 201, response: group.children[0], body: { ...metadata, label: "scene" } },
    { name: "list containers", call: (c) => c.listContainers({ universeId: 5 }), path: "/universes/5/containers", method: "GET", status: 200, response: [container] },
    { name: "list loose pieces", call: (c) => c.listLoosePieces({ universeId: 5 }), path: "/universes/5/pieces?container=none", method: "GET", status: 200, response: [piece] },
    { name: "list container pieces", call: (c) => c.listContainerPieces({ containerId: 1 }), path: "/containers/1/pieces", method: "GET", status: 200, response: [piece] },
    { name: "get leaf piece", call: (c) => c.getPiece(pieceScope), path: "/pieces/9", method: "GET", status: 200, response: pieceDetail },
    { name: "delete piece", call: (c) => c.deletePiece(pieceScope), path: "/pieces/9", method: "DELETE", status: 204 },
    { name: "restore piece", call: (c) => c.restorePiece(pieceScope), path: "/pieces/9/restore", method: "POST", status: 200, response: pieceDetail },
    { name: "get group piece", call: (c) => c.getPiece({ pieceId: 10 }), path: "/pieces/10", method: "GET", status: 200, response: group },
    { name: "get piece deltas", call: (c) => c.getDeltas(pieceScope, 2), path: "/pieces/9/deltas?since=2", method: "GET", status: 200, response: [revision] },
    { name: "post piece delta", call: (c) => c.postDelta(pieceScope, submission), path: "/pieces/9/deltas", method: "POST", status: 200, response: revision, body: submission },
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

test("delete and restore reject invalid responses and preserve errors", async () => {
  const calls: Array<(client: Client) => Promise<unknown>> = [
    c => c.deleteUniverse({ universeId: 5 }), c => c.deleteContainer({ containerId: 1 }), c => c.deletePiece(pieceScope),
    c => c.restoreUniverse({ universeId: 5 }), c => c.restoreContainer({ containerId: 1 }), c => c.restorePiece(pieceScope),
  ];
  for (const call of calls) {
    const invalid = createManuscriptClient({ fetch: async () => Response.json({}, { status: 200 }) });
    await expect(call(invalid)).rejects.toMatchObject({ status: 200, kind: "invalid_response" });
    const rejected = createManuscriptClient({ fetch: async () => Response.json({ error: "Missing", code: "NOT_FOUND" }, { status: 404 }) });
    await expect(call(rejected)).rejects.toMatchObject({ status: 404, kind: "http", code: "NOT_FOUND" });
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

test("all create methods parse responses and preserve structured failures", async () => {
  const cases: Array<[(client: Client) => Promise<unknown>, unknown]> = [
    [(c) => c.createSeries(metadata), series],
    [(c) => c.createBook(scope, metadata), book],
    [(c) => c.createChapter(scope, { ...metadata, kind: "chapter" }), chapter],
  ];
  for (const [call, resource] of cases) {
    const valid = createManuscriptClient({ fetch: async () => Response.json(resource, { status: 201 }) });
    expect(await call(valid)).toEqual(resource);
    const malformed = createManuscriptClient({ fetch: async () => Response.json({ ...resource as object, title: " " }, { status: 201 }) });
    await expect(call(malformed)).rejects.toMatchObject({ status: 201, kind: "invalid_response", code: "INVALID_RESPONSE" });
    const rejected = createManuscriptClient({ fetch: async () => Response.json({ error: "Invalid title", code: "VALIDATION_ERROR" }, { status: 422 }) });
    await expect(call(rejected)).rejects.toMatchObject({ status: 422, kind: "http", code: "VALIDATION_ERROR", message: "Invalid title" });
  }
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
      expect(error).toMatchObject({ message: code === "SQLX" ? "Couldn't reach your manuscript's storage. Your text is still here; try again." : "backend message", status, code, kind: "http", ...(code === "SQLX" ? { detail: "backend message" } : {}) });
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
