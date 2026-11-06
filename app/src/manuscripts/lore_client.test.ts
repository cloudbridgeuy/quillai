import { expect, test } from "bun:test";
import { ManuscriptApiError } from "./client";
import { createLoreClient, parseLoreEntry, parseTimelinePoint } from "./lore_client";
import { newLoreDraft } from "./lore_panel";

const entry = { ...newLoreDraft(), id: 1, universe_id: 2, name: "Mira", hidden_notes: "Author secret" };

test("parses author lore without losing fields", () => {
  expect(parseLoreEntry(entry)).toEqual(entry);
});

test("rejects malformed lore fields", () => {
  for (const patch of [{ id: 0 }, { universe_id: 1.5 }, { name: " " }, { pinned: 1 }, { kind: "person" },
    { level: "unknown" }, { keywords: [""] }, { aliases: "M" }, { hidden_notes: null }, { visible_text: 1 }]) {
    expect(() => parseLoreEntry({ ...entry, ...patch })).toThrow();
  }
});

test("lore client uses scoped CRUD paths, bodies and statuses", async () => {
  const calls: [string, RequestInit | undefined][] = [];
  const client = createLoreClient({ baseUrl: "http://localhost:2908/", fetch: async (url, init) => {
    calls.push([url, init]);
    if (init?.method === "DELETE") return new Response(null, { status: 204 });
    return Response.json(init?.method === "GET" && url.endsWith("/lore") ? [entry] : entry,
      { status: init?.method === "POST" ? 201 : 200 });
  } });
  expect(await client.list({ universeId: 2 })).toEqual([entry]);
  expect(await client.get({ universeId: 2, loreId: 1 })).toEqual(entry);
  expect(await client.create({ universeId: 2 }, { ...newLoreDraft(), name: "Mira" })).toEqual(entry);
  await client.update({ universeId: 2, loreId: 1 }, { pinned: false });
  await client.delete({ universeId: 2, loreId: 1 });
  expect(calls.map(([url, init]) => [url, init?.method])).toEqual([
    ["http://localhost:2908/universes/2/lore", "GET"], ["http://localhost:2908/universes/2/lore/1", "GET"],
    ["http://localhost:2908/universes/2/lore", "POST"], ["http://localhost:2908/universes/2/lore/1", "PUT"],
    ["http://localhost:2908/universes/2/lore/1", "DELETE"],
  ]);
  expect(JSON.parse(String(calls[3]?.[1]?.body))).toEqual({ pinned: false });
});

test("lore client resolves a series to its universe through the container route", async () => {
  const urls: string[] = [];
  const client = createLoreClient({ baseUrl: "http://localhost:2908", fetch: async url => {
    urls.push(url);
    return Response.json({ id: 4, universeId: 9, title: "Saga", position: 0, kind: "book_series" });
  } });
  expect(await client.seriesUniverse({ seriesId: 4 })).toBe(9);
  expect(urls).toEqual(["http://localhost:2908/containers/4"]);
});

test("lore client reports structured HTTP and invalid-response errors", async () => {
  for (const [response, kind, code] of [
    [Response.json({ error: "Missing lore", code: "NOT_FOUND" }, { status: 404 }), "http", "NOT_FOUND"],
    [new Response("broken", { status: 500 }), "invalid_response", "INVALID_RESPONSE"],
    [Response.json({}), "invalid_response", "INVALID_RESPONSE"],
    [Response.json([], { status: 201 }), "invalid_response", "INVALID_RESPONSE"],
  ] as const) {
    const client = createLoreClient({ fetch: async () => response });
    try { await client.list({ universeId: 2 }); throw new Error("Expected failure"); } catch (error) {
      expect(error).toBeInstanceOf(ManuscriptApiError);
      expect(error).toMatchObject({ status: response.status, kind, code });
    }
  }
});

test("parses active lore states and rejects malformed ones", () => {
  const state = { id: 3, name: "Older", pointId: 2, visibleText: "v", hiddenNotes: "h" };
  expect(parseLoreEntry({ ...entry, states: [state] }).states).toEqual([state]);
  expect(() => parseLoreEntry({ ...entry, states: [{ ...state, pointId: "2" }] })).toThrow();
});

test("timeline client uses point paths, bodies and statuses", async () => {
  const point = { id: 5, universeId: 2, name: "Act one", position: 0 };
  const calls: [string, RequestInit | undefined][] = [];
  const client = createLoreClient({ baseUrl: "http://localhost:2908", fetch: async (url, init) => {
    calls.push([url, init]);
    if (init?.method === "DELETE") return new Response(null, { status: 204 });
    if (url.endsWith("/pieces/7")) return Response.json({ parentId: null, timelinePointId: 5 });
    return Response.json(point, { status: init?.method === "POST" ? 201 : 200 });
  } });
  expect(await client.createPoint({ universeId: 2 }, { name: "Act one", position: 0 })).toEqual({ id: 5, name: "Act one", position: 0 });
  await client.updatePoint({ pointId: 5 }, { name: "Act 1" });
  await client.deletePoint({ pointId: 5 });
  await client.setPiecePoint({ pieceId: 7 }, null);
  expect(calls.map(([url, init]) => [init?.method, url.replace("http://localhost:2908", "")])).toEqual([
    ["POST", "/universes/2/timeline"], ["PUT", "/timeline/5"], ["DELETE", "/timeline/5"], ["PUT", "/pieces/7"],
  ]);
  expect(JSON.parse(String(calls[1]![1]!.body))).toEqual({ name: "Act 1" });
  expect(JSON.parse(String(calls[3]![1]!.body))).toEqual({ timelinePointId: null });
  expect(() => parseTimelinePoint({ id: 1, position: 0 })).toThrow();
});

test("state client uses state paths, bodies and statuses and reports a 409 conflict", async () => {
  const state = { id: 9, name: "Older", pointId: 5, visibleText: "v", hiddenNotes: "h" };
  const calls: [string, RequestInit | undefined][] = [];
  let conflict = false;
  const client = createLoreClient({ baseUrl: "http://localhost:2908", fetch: async (url, init) => {
    calls.push([url, init]);
    if (conflict) return Response.json({ error: "dup", code: "CONFLICT" }, { status: 409 });
    if (init?.method === "DELETE") return new Response(null, { status: 204 });
    if (!init?.method || init.method === "GET") return Response.json([state]);
    return Response.json(state, { status: init.method === "POST" ? 201 : 200 });
  } });
  const scope = { universeId: 2, loreId: 4 };
  expect(await client.listStates(scope)).toEqual([state]);
  expect(await client.createState(scope, { name: "Older", pointId: 5, visibleText: "v", hiddenNotes: "h" })).toEqual(state);
  await client.updateState({ ...scope, stateId: 9 }, { name: "Newer" });
  await client.deleteState({ ...scope, stateId: 9 });
  expect(calls.map(([url, init]) => [init?.method ?? "GET", url.replace("http://localhost:2908", "")])).toEqual([
    ["GET", "/universes/2/lore/4/states"], ["POST", "/universes/2/lore/4/states"],
    ["PUT", "/universes/2/lore/4/states/9"], ["DELETE", "/universes/2/lore/4/states/9"],
  ]);
  expect(JSON.parse(String(calls[2]![1]!.body))).toEqual({ name: "Newer" });
  conflict = true;
  try { await client.createState(scope, { name: "x", pointId: 5 }); throw new Error("Expected failure"); } catch (error) {
    expect(error).toMatchObject({ status: 409, code: "CONFLICT" });
  }
});
