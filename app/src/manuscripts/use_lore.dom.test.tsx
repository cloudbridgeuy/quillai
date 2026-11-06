import { afterEach, expect, test } from "bun:test";
import { act, useState } from "react";
import { cleanup, render } from "../test/dom";
import type { EditorScope } from "./contract";
import { newLoreDraft } from "./lore_panel";
import { useLore } from "./use_lore";

const entry = { ...newLoreDraft(), id: 1, universe_id: 2, name: "Mira", visible_text: "Base text.",
  states: [{ id: 9, name: "Later", pointId: 5, visibleText: "State text.", hiddenNotes: "" }] };
const realFetch = globalThis.fetch;
let latest: ReturnType<typeof useLore> | null = null;

let setScope: (scope: EditorScope) => void = () => {};

function Probe({ initial }: { initial: EditorScope }) {
  const [scope, set] = useState(initial);
  setScope = set;
  latest = useLore(scope);
  return null;
}

function stubFetch(routes: Record<string, () => Response>): string[] {
  const calls: string[] = [];
  globalThis.fetch = (async (url: string) => {
    const path = new URL(url).pathname;
    calls.push(path);
    return (routes[path] ?? (() => Response.json({ error: "missing", code: "NOT_FOUND" }, { status: 404 })))();
  }) as typeof fetch;
  return calls;
}

const flush = () => act(async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); });

afterEach(async () => {
  await cleanup();
  globalThis.fetch = realFetch;
  latest = null;
});

test("timeline and piece failures leave lore ready with base text", async () => {
  stubFetch({ "/universes/2/lore": () => Response.json([entry]) });
  await render(<Probe initial={{ universeId: 2, pieceId: 7 }} />);
  await flush();
  expect(latest!.status).toBe("ready");
  expect(latest!.entries.map(value => value.id)).toEqual([1]);
  expect(latest!.resolved.map(value => [value.visible_text, value.stateName])).toEqual([["Base text.", null]]);
});

test("switching piece keeps lore, refetches only the piece point and resolves its state", async () => {
  const calls = stubFetch({
    "/universes/2/lore": () => Response.json([entry]),
    "/universes/2/timeline": () => Response.json([{ id: 5, name: "Act", position: 1 }]),
    "/pieces/7": () => Response.json({ parentId: null, timelinePointId: null }),
    "/pieces/8": () => Response.json({ parentId: null, timelinePointId: 5 }),
  });
  await render(<Probe initial={{ universeId: 2, pieceId: 7 }} />);
  await flush();
  expect(latest!.resolved[0]!.stateName).toBeNull();
  await act(async () => setScope({ universeId: 2, pieceId: 8 }));
  expect(latest!.status).toBe("ready");
  expect(latest!.entries.map(value => value.id)).toEqual([1]);
  await flush();
  expect(latest!.resolved[0]!.visible_text).toBe("State text.");
  expect(calls.filter(path => path === "/universes/2/lore")).toHaveLength(1);
  expect(calls.filter(path => path === "/universes/2/timeline")).toHaveLength(1);
});
