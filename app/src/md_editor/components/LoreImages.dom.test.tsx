import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, render } from "../../test/dom";
import { formatMessage } from "../../i18n/messages";
import type { LoreEntry } from "../../manuscripts/lore_client";
import { useLore } from "../../manuscripts/use_lore";
import { LoreImages } from "./LoreImages";

const realFetch = globalThis.fetch;
const text = (key: Parameters<typeof formatMessage>[1]) => formatMessage("en", key);

function entry(kind: LoreEntry["kind"], level: LoreEntry["level"]): LoreEntry {
  return { id: 9, universe_id: 2, kind, name: "Harbor", keywords: [], aliases: [], visible_text: "Salt docks.",
    hidden_notes: "", pinned: false, level, states: [] };
}

function Harness({ value }: { value: LoreEntry }) {
  return <LoreImages lore={useLore({ universeId: 2, pieceId: 7 })} entry={value} disabled={false} />;
}

function stub() {
  const posts: string[] = [];
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    if (init?.method === "POST") {
      posts.push(String(init.body));
      return Response.json({ id: posts.length, timelinePointId: null }, { status: 201 });
    }
    if (pathname.endsWith("/images") || pathname.endsWith("/lore") || pathname.endsWith("/timeline")) return Response.json([]);
    if (pathname === "/pieces/7") return Response.json({ parentId: null, timelinePointId: null });
    return Response.json({ error: "missing", code: "NOT_FOUND" }, { status: 404 });
  }) as typeof fetch;
  return posts;
}

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const select = (label: string) => [...document.querySelectorAll("label")]
  .find(node => node.querySelector("span")?.textContent === label)?.querySelector("select") ?? undefined;
const choose = (label: string, value: string) => act(async () => {
  const node = select(label)!;
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(node, value);
  node.dispatchEvent(new Event("change", { bubbles: true }));
});
const generateButton = () => [...document.querySelectorAll("button")].find(node => node.textContent === text("lore.images.generate"));
const generate = async () => { await act(async () => generateButton()!.click()); await flush(); };

afterEach(() => { cleanup(); globalThis.fetch = realFetch; });

test("place entries choose portrait, map or render with a time of day", async () => {
  const posts = stub();
  await render(<Harness value={entry("place", "include")} />);
  await flush();
  const purpose = select(text("lore.images.purpose"));
  expect([...(purpose?.options ?? [])].map(option => option.value)).toEqual(["portrait", "map", "render"]);
  expect(select(text("lore.images.timeOfDay"))).toBeUndefined();
  await generate();
  await choose(text("lore.images.purpose"), "map");
  expect(select(text("lore.images.timeOfDay"))).toBeUndefined();
  await generate();
  await choose(text("lore.images.purpose"), "render");
  expect(select(text("lore.images.timeOfDay"))?.value).toBe("day");
  await generate();
  await choose(text("lore.images.timeOfDay"), "dusk");
  await generate();
  await choose(text("lore.images.purpose"), "map");
  await generate();
  expect(posts).toEqual([
    `{"purpose":"portrait"}`, `{"purpose":"map"}`, `{"purpose":"render","timeOfDay":"day"}`,
    `{"purpose":"render","timeOfDay":"dusk"}`, `{"purpose":"map"}`,
  ]);
});

test("other kinds keep one Generate button and send no purpose", async () => {
  const posts = stub();
  await render(<Harness value={entry("character", "include")} />);
  await flush();
  expect(document.querySelector("select")).toBeNull();
  await generate();
  expect(posts).toEqual(["{}"]);
});

test("Leave out entries show no Generate button", async () => {
  stub();
  await render(<Harness value={entry("place", "leave_out")} />);
  await flush();
  expect(generateButton()).toBeUndefined();
  expect(document.querySelector("select")).toBeNull();
});
