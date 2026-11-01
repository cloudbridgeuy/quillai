import { expect, test } from "bun:test";
import { createTrashClient, type TrashItem } from "./trash_client";

const item: TrashItem = {
  type: "lore", id: 7, title: "Name", deletedAt: "2026-01-01 12:00:00",
  location: { universe: "World", series: null, book: null },
  childCounts: { chapters: 0, pieces: 0, lore: 0 }, deletedParents: ["World"],
  resourcePath: "/universes/1/lore/7", restoreAncestors: ["/universes/1"],
};

test("parses trash and orders ancestor restores before both actions", async () => {
  const calls: string[] = [];
  const client = createTrashClient({ baseUrl: "http://localhost", fetch: async (url, init) => {
    calls.push(`${init?.method} ${url}`);
    if (url.endsWith("/trash")) return Response.json([item]);
    if (init?.method === "DELETE") return new Response(null, { status: 204 });
    return Response.json({ id: 1 });
  } });
  const [entry] = await client.list();
  expect(entry).toEqual(item);
  await client.restore(entry!);
  await client.deleteForever(entry!);
  expect(calls).toEqual([
    "GET http://localhost/trash", "POST http://localhost/universes/1/restore",
    "POST http://localhost/universes/1/lore/7/restore", "POST http://localhost/universes/1/restore",
    "DELETE http://localhost/universes/1/lore/7?permanent=true",
  ]);
});

test("rejects malformed items, counts, paths, and API errors", async () => {
  for (const invalid of [{ ...item, type: "unknown" }, { ...item, id: 0 }, { ...item, childCounts: { ...item.childCounts, chapters: -1 } },
    { ...item, resourcePath: "//example.com" }, { ...item, deletedAt: "bad" }]) {
    const client = createTrashClient({ fetch: async () => Response.json([invalid]) });
    await expect(client.list()).rejects.toMatchObject({ kind: "invalid_response" });
  }
  const client = createTrashClient({ fetch: async () => Response.json({ error: "Missing", code: "NOT_FOUND" }, { status: 404 }) });
  await expect(client.list()).rejects.toMatchObject({ status: 404, kind: "http" });
});

test("ancestor failure prevents permanent delete", async () => {
  const calls: string[] = [];
  const client = createTrashClient({ fetch: async (url, init) => {
    calls.push(`${init?.method} ${url}`);
    return Response.json({ error: "Missing", code: "NOT_FOUND" }, { status: 404 });
  } });
  await expect(client.deleteForever(item)).rejects.toMatchObject({ status: 404 });
  expect(calls).toHaveLength(1);
  expect(calls[0]).toEndWith("/universes/1/restore");
});
