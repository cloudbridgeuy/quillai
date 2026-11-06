import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, render } from "../../test/dom";
import { useLore } from "../../manuscripts/use_lore";
import { LorePanel } from "./LorePanel";

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
