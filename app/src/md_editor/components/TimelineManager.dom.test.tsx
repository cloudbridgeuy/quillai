import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, click, render, type } from "../../test/dom";
import { useLore } from "../../manuscripts/use_lore";
import { TimelineManager } from "./TimelineManager";

const realFetch = globalThis.fetch;
type Point = { id: number; name: string; position: number };

function Harness() {
  const lore = useLore({ universeId: 2, pieceId: 7 });
  return lore.status === "ready" ? <TimelineManager lore={lore} disabled={false} /> : null;
}

function stub(initial: Point[], failOn: (call: string) => boolean = () => false) {
  const calls: string[] = [];
  let points = initial;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    const call = `${method} ${pathname}`;
    calls.push(init?.body ? `${call} ${init.body}` : call);
    if (failOn(call)) return Response.json({ error: "boom", code: "INTERNAL" }, { status: 500 });
    if (pathname === "/universes/2/lore") return Response.json([]);
    if (pathname === "/universes/2/timeline" && method === "GET") return Response.json(points);
    if (pathname === "/universes/2/timeline") {
      const created = { id: 99, ...JSON.parse(String(init!.body)) };
      points = [...points, created];
      return Response.json(created, { status: 201 });
    }
    if (pathname.startsWith("/timeline/") && method === "PUT") {
      const id = Number(pathname.split("/")[2]);
      const patch = JSON.parse(String(init!.body));
      points = points.map(point => point.id === id ? { ...point, ...patch } : point);
      return Response.json(points.find(point => point.id === id));
    }
    if (pathname.startsWith("/timeline/") && method === "DELETE") {
      points = points.filter(point => point.id !== Number(pathname.split("/")[2]));
      return new Response(null, { status: 204 });
    }
    return Response.json({ parentId: null, timelinePointId: null });
  }) as typeof fetch;
  return calls;
}

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const button = (name: string) => [...document.querySelectorAll("button")].find(item => item.textContent === name || item.getAttribute("aria-label") === name) as HTMLButtonElement;
const open = async () => {
  await render(<Harness />);
  await flush();
  await click(button("Timeline"));
  await flush();
};

afterEach(async () => {
  await cleanup();
  globalThis.fetch = realFetch;
});

test("adds a point after the last position", async () => {
  const calls = stub([{ id: 1, name: "A", position: 10 }]);
  await open();
  await type(document.querySelector("input")!, "B");
  await click(button("Add point"));
  await flush();
  expect(calls.some(call => call.startsWith("POST /universes/2/timeline") && call.includes('"name":"B"'))).toBe(true);
  expect(button("B")).toBeTruthy();
});

test("renames the selected point", async () => {
  const calls = stub([{ id: 1, name: "A", position: 10 }]);
  await open();
  await click(button("A"));
  await type(document.querySelector("input")!, "Renamed");
  await click(button("Rename point"));
  await flush();
  expect(calls).toContain('PUT /timeline/1 {"name":"Renamed"}');
  expect(button("Renamed")).toBeTruthy();
});

test("delete needs a second confirming click", async () => {
  const calls = stub([{ id: 1, name: "A", position: 10 }]);
  await open();
  await click(button("A"));
  await click(button("Delete point"));
  expect(calls.some(call => call.startsWith("DELETE"))).toBe(false);
  await click(button("Confirm delete"));
  await flush();
  expect(calls).toContain("DELETE /timeline/1");
  expect(document.body.textContent).toContain("No timeline points yet.");
});

test("move down swaps positions with two PUTs", async () => {
  const calls = stub([{ id: 1, name: "A", position: 10 }, { id: 2, name: "B", position: 20 }]);
  await open();
  await click(button("Move A down"));
  await flush();
  const puts = calls.filter(call => call.startsWith("PUT /timeline/"));
  expect(puts).toEqual(['PUT /timeline/2 {"position":0}', 'PUT /timeline/1 {"position":1}']);
  expect([...document.querySelectorAll("li")].map(item => item.textContent?.[0])).toEqual(["B", "A"]);
});

test("a failed second reorder PUT shows an error and still refetches the timeline", async () => {
  const calls = stub([{ id: 1, name: "A", position: 10 }, { id: 2, name: "B", position: 20 }], call => call === "PUT /timeline/2");
  await open();
  const before = calls.filter(call => call === "GET /universes/2/timeline").length;
  await click(button("Move B up"));
  await flush();
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(calls.filter(call => call === "GET /universes/2/timeline").length).toBe(before + 1);
});
