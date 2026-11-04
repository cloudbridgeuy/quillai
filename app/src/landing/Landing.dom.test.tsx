import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, click, render } from "../test/dom";
import { Landing } from "./Landing";
import { RECENT_KEY } from "./recent";

const fetchOriginal = globalThis.fetch;
afterEach(async () => { await cleanup(); globalThis.fetch = fetchOriginal; window.localStorage.clear(); });

test("shows recent rows and forget persists", async () => {
  const items = [1, 2].map(id => ({ scope: { universeId: 1, pieceId: id }, title: `Piece ${id}` }));
  window.localStorage.setItem(RECENT_KEY, JSON.stringify(items));
  globalThis.fetch = Object.assign(async () => { throw new Error("offline"); }, { preconnect: fetchOriginal.preconnect });
  const container = await render(<Landing />);
  const section = container.querySelector("#landing-resume")!.parentElement!;
  expect(section.textContent).toContain("Piece 1");
  expect(section.textContent).toContain("Piece 2");
  expect([...section.querySelectorAll("button")].filter(button => button.textContent === "Open")).toHaveLength(2);
  await click([...section.querySelectorAll("button")].find(button => button.textContent === "Forget")!);
  expect(section.textContent).not.toContain("Piece 1");
  expect(section.textContent).toContain("Piece 2");
  expect(JSON.parse(window.localStorage.getItem(RECENT_KEY)!)).toEqual([items[1]]);
});

test("forget during a pending check stays removed after resolution", async () => {
  const items = [1, 2].map(id => ({ scope: { universeId: 1, pieceId: id }, title: `Piece ${id}` }));
  window.localStorage.setItem(RECENT_KEY, JSON.stringify(items));
  const pending = Promise.withResolvers<Response>();
  globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
    if (String(input).endsWith("/pieces/1")) return pending.promise;
    throw new Error("offline");
  }, { preconnect: fetchOriginal.preconnect });
  const container = await render(<Landing />);
  const section = container.querySelector("#landing-resume")!.parentElement!;
  await click([...section.querySelectorAll("button")].find(button => button.textContent === "Forget")!);
  await act(async () => pending.resolve(Response.json({ error: "not found", code: "NOT_FOUND" }, { status: 404 })));
  expect(section.textContent).not.toContain("Piece 1");
  expect(section.textContent).toContain("Piece 2");
  expect(JSON.parse(window.localStorage.getItem(RECENT_KEY)!)).toEqual([items[1]]);
});

test("missing rows leave storage while remaining rows resolve current titles", async () => {
  const items = [1, 2].map(id => ({ scope: { universeId: 1, pieceId: id }, title: `Piece ${id}` }));
  window.localStorage.setItem(RECENT_KEY, JSON.stringify(items));
  globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
    if (String(input).endsWith("/pieces/1")) return Response.json({ error: "not found", code: "NOT_FOUND" }, { status: 404 });
    if (String(input).endsWith("/pieces/2")) return Response.json({ id: 2, universeId: 1, containerId: null, parentId: null, title: "Current title", position: 0, label: "short_story", format: "prose", shape: "leaf", content: { ops: [] }, headRev: 0 });
    return Response.json([]);
  }, { preconnect: fetchOriginal.preconnect });
  const container = await render(<Landing />);
  const section = container.querySelector("#landing-resume")!.parentElement!;
  expect(section.textContent).not.toContain("Piece 1");
  expect(section.textContent).toContain("Current title");
  expect(JSON.parse(window.localStorage.getItem(RECENT_KEY)!)).toEqual([items[1]]);
});
