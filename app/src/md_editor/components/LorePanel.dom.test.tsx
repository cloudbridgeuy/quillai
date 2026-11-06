import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, render } from "../../test/dom";
import { useLore } from "../../manuscripts/use_lore";
import { LorePanel } from "./LorePanel";
import { formatMessage } from "../../i18n/messages";

const realFetch = globalThis.fetch;
const points = [{ id: 5, name: "Act", position: 1 }, { id: 6, name: "End", position: 2 }];

let latest: ReturnType<typeof useLore> | null = null;
function Harness() {
  const lore = useLore({ universeId: 2, pieceId: 7 });
  latest = lore;
  return <LorePanel lore={lore} disabled={false} pieceId={7} />;
}

function stub({ failRefetch = false, failPut = false, parentPoint = null as number | null } = {}) {
  const calls: string[] = [];
  let putDone = false;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    calls.push(init?.body ? `${method} ${pathname} ${init.body}` : `${method} ${pathname}`);
    if (pathname === "/universes/2/lore") return Response.json([]);
    if (pathname === "/universes/2/timeline") return Response.json(points);
    if (pathname === "/pieces/7" && method === "PUT") {
      if (failPut) return Response.json({ error: "boom", code: "INTERNAL" }, { status: 500 });
      putDone = true;
      return Response.json({ parentId: null, timelinePointId: JSON.parse(String(init!.body)).timelinePointId });
    }
    if (pathname === "/pieces/7" && failRefetch && putDone) return Response.json({ error: "boom", code: "INTERNAL" }, { status: 500 });
    if (pathname === "/pieces/3") return Response.json({ parentId: null, timelinePointId: parentPoint });
    return Response.json({ parentId: parentPoint === null ? null : 3, timelinePointId: null });
  }) as typeof fetch;
  return calls;
}

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const picker = () => document.querySelector("select") as HTMLSelectElement;
const choose = (value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(picker(), value);
  picker().dispatchEvent(new Event("change", { bubbles: true }));
});

afterEach(async () => {
  await cleanup();
  globalThis.fetch = realFetch;
  latest = null;
});

test("picker lists the base option first and PUTs the chosen point, then null", async () => {
  const calls = stub();
  await render(<Harness />);
  await flush();
  expect([...picker().options].map(option => option.textContent)).toEqual(["No point (base text)", "Act", "End"]);
  await choose("6");
  await flush();
  expect(calls).toContain('PUT /pieces/7 {"timelinePointId":6}');
  expect(picker().value).toBe("6");
  await choose("");
  await flush();
  expect(calls).toContain('PUT /pieces/7 {"timelinePointId":null}');
  expect(picker().value).toBe("");
});

test("the chosen point sticks when the piece refetch fails", async () => {
  stub({ failRefetch: true });
  await render(<Harness />);
  await flush();
  await choose("5");
  await flush();
  expect(picker().value).toBe("5");
  expect(latest!.piece?.timelinePointId).toBe(5);
});

test("a failed PUT shows an alert and keeps the previous value", async () => {
  stub({ failPut: true });
  await render(<Harness />);
  await flush();
  await choose("5");
  await flush();
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(picker().value).toBe("");
});

test("the empty option inherits the parent's point until the piece sets its own", async () => {
  stub({ parentPoint: 6 });
  await render(<Harness />);
  await flush();
  expect(picker().options[0]!.textContent).toBe("Inherit: End");
  await choose("5");
  await flush();
  expect(picker().options[0]!.textContent).toBe("No point (base text)");
});

type StateRow = { id: number; name: string; pointId: number; visibleText: string; hiddenNotes: string };

function stubStates({ states = [] as StateRow[], conflict = false } = {}) {
  const calls: string[] = [];
  let current = states;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    calls.push(init?.body ? `${method} ${pathname} ${init.body}` : `${method} ${pathname}`);
    const entry = { id: 4, universe_id: 2, kind: "character", level: "include", pinned: false, name: "Mira", keywords: [], aliases: [],
      visible_text: "base", hidden_notes: "", states: current };
    if (pathname === "/universes/2/lore") return Response.json([entry]);
    if (pathname === "/universes/2/lore/4") return Response.json(entry);
    if (pathname === "/universes/2/timeline") return Response.json(points);
    if (pathname.startsWith("/pieces/")) return Response.json({ parentId: null, timelinePointId: null });
    if (pathname.startsWith("/universes/2/lore/4/states")) {
      if (method !== "GET" && conflict) return Response.json({ error: "dup", code: "CONFLICT" }, { status: 409 });
      const body = init?.body ? JSON.parse(String(init.body)) : {};
      if (method === "POST") { current = [...current, { id: 30, visibleText: "", hiddenNotes: "", ...body }]; return Response.json(current.at(-1), { status: 201 }); }
      if (method === "PUT") { current = current.map(row => ({ ...row, ...body })); return Response.json(current[0]); }
      if (method === "DELETE") { current = []; return new Response(null, { status: 204 }); }
    }
    return Response.json([]);
  }) as typeof fetch;
  return calls;
}

const byLabel = (label: string) => [...document.querySelectorAll("label")].find(node => node.firstElementChild?.textContent === label)!.control as HTMLInputElement;
const button = (name: string) => [...document.querySelectorAll("button")].find(node => node.textContent === name || node.getAttribute("aria-label") === name) as HTMLButtonElement;
const typeInto = (label: string, value: string) => act(async () => {
  const field = byLabel(label);
  Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")!.set!.call(field, value);
  field.dispatchEvent(new Event(field instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
});
const press = (name: string) => act(async () => button(name).click());
const openEntry = async () => { await render(<Harness />); await flush(); await flush(); await press("Mira · character"); await flush(); };

test("states section adds a state at a point and refreshes the list", async () => {
  const calls = stubStates();
  await openEntry();
  expect(document.body.textContent).toContain("No states yet.");
  await typeInto("State name", "Younger");
  await typeInto("State text", "young text");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  expect(calls).toContain('POST /universes/2/lore/4/states {"name":"Younger","pointId":5,"visibleText":"young text","hiddenNotes":""}');
  expect(document.body.textContent).toContain("Younger");
  expect(document.body.textContent).not.toContain("No states yet.");
});

test("states section edits a state with PUT", async () => {
  const calls = stubStates({ states: [{ id: 8, name: "Older", pointId: 6, visibleText: "old", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  expect(byLabel("State name").value).toBe("Older");
  await typeInto("State name", "Oldest");
  await press("Save state");
  await flush();
  expect(calls).toContain('PUT /universes/2/lore/4/states/8 {"name":"Oldest","pointId":6,"visibleText":"old","hiddenNotes":""}');
});

test("states section deletes with a two-click confirmation", async () => {
  const calls = stubStates({ states: [{ id: 8, name: "Older", pointId: 6, visibleText: "old", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  await press("Delete state");
  expect(calls.some(call => call.startsWith("DELETE"))).toBe(false);
  await press("Confirm delete");
  await flush();
  expect(calls).toContain("DELETE /universes/2/lore/4/states/8");
  expect(document.body.textContent).toContain("No states yet.");
});

test("a 409 shows the states-specific message", async () => {
  stubStates({ conflict: true });
  await openEntry();
  await typeInto("State name", "Dup");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  expect(document.body.textContent).toContain("This entry already has a state at that point.");
});

test("states strings exist in English and Spanish", () => {
  for (const key of ["lore.states.title", "lore.states.empty", "lore.states.conflict", "lore.states.add"] as const) {
    expect(formatMessage("en", key)).not.toBe(key);
    expect(formatMessage("es", key)).not.toBe(formatMessage("en", key));
  }
  expect(formatMessage("es", "lore.states.conflict")).toBe("Esta entrada ya tiene un estado en ese punto.");
});
