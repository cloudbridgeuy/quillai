import { afterEach, expect, test } from "bun:test";
import { useRef, useState, type RefObject } from "react";
import type Quill from "quill";
import { act } from "react";
import { cleanup, click, render, type } from "../test/dom";
import { LAST_EDITED_KEY } from "../landing/routes";
import { RECENT_KEY } from "../landing/recent";
import { apiErrorMessage } from "../api_error";
import { ManuscriptTree } from "./components/ManuscriptTree";
import { useChapterSync } from "./use_chapter_sync";

const fetchOriginal = globalThis.fetch;
const assignOriginal = window.location.assign.bind(window.location);
const assigned: string[] = [];
afterEach(async () => {
  await cleanup();
  globalThis.fetch = fetchOriginal;
  window.location.assign = assignOriginal;
  assigned.splice(0);
  window.localStorage.clear();
  window.location.href = "about:blank";
});

function installFetch(handler: (url: string, init?: RequestInit) => Response | Promise<Response>) {
  globalThis.fetch = Object.assign(async (input: RequestInfo | URL, init?: RequestInit) => handler(String(input), init), { preconnect: fetchOriginal.preconnect });
}

const chapter = { scope: { seriesId: 4, bookId: 5, chapterId: 6 }, title: "Gone chapter" };
const piece = { scope: { universeId: 9, pieceId: 9 }, title: "Gone piece" };

function Probe({ quillRef, start, status }: {
  quillRef: RefObject<Quill | null>;
  start: RefObject<number | null>;
  status: RefObject<string | null>;
}) {
  const sync = useChapterSync(quillRef, () => false, () => {}, start, status);
  return <ManuscriptTree selectedChapter={sync.scope} disabled={!sync.ready && !sync.needsChapter} onSelect={() => {}} />;
}

function Harness() {
  const quillRef = useRef<Quill | null>(null);
  const start = useRef<number | null>(null);
  const status = useRef<string | null>(null);
  return <Probe quillRef={quillRef} start={start} status={status} />;
}

async function openStaleChapter() {
  window.location.href = "http://localhost/?seriesId=4&bookId=5&chapterId=6";
  window.location.assign = (url: string | URL) => { assigned.push(String(url)); };
  window.localStorage.setItem(RECENT_KEY, JSON.stringify([chapter, piece]));
  window.localStorage.setItem(LAST_EDITED_KEY, JSON.stringify(chapter));
  installFetch(url => url.endsWith("/tree")
    ? Response.json({ series: [], universes: [] })
    : Response.json({ error: "missing", code: "NOT_FOUND" }, { status: 404 }));
  const container = await render(<Harness />);
  await act(async () => { await Promise.resolve(); });
  return container;
}

test("a stale chapter URL enables + Universe and drops the saved ids", async () => {
  const container = await openStaleChapter();
  const button = [...container.querySelectorAll("button")].find(item => item.textContent === "+ Universe");
  expect(button?.disabled).toBe(false);
  await click(button!);
  expect(container.querySelector("[aria-label='Universe title']")).not.toBeNull();
  expect(JSON.parse(window.localStorage.getItem(RECENT_KEY)!)).toEqual([piece]);
  expect(window.localStorage.getItem(LAST_EDITED_KEY)).toBeNull();
  expect(assigned.some(url => !new URL(url).searchParams.has("seriesId"))).toBe(true);
});

function Lockable() {
  const [disabled, setDisabled] = useState(false);
  return <div>
    <button type="button" onClick={() => setDisabled(true)}>Lock</button>
    <ManuscriptTree selectedChapter={null} disabled={disabled} onSelect={() => {}} />
  </div>;
}

test("creating a universe shows an api error when the action cannot run", async () => {
  installFetch(() => Response.json({ series: [], universes: [] }));
  const container = await render(<Lockable />);
  await click([...container.querySelectorAll("button")].find(item => item.textContent === "+ Universe")!);
  await click([...container.querySelectorAll("button")].find(item => item.textContent === "Lock")!);
  const form = container.querySelector("form")!;
  await act(async () => { form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })); });
  expect(container.textContent).toContain(apiErrorMessage("", ""));
});

test("a failed universe create shows the api error instead of a parser message", async () => {
  installFetch((_url, init) => init?.method === "POST"
    ? Response.json({ id: 1 }, { status: 201 })
    : Response.json({ series: [], universes: [] }));
  const container = await render(<ManuscriptTree selectedChapter={null} disabled={false} onSelect={() => {}} />);
  await click([...container.querySelectorAll("button")].find(item => item.textContent === "+ Universe")!);
  await type(container.querySelector("input")!, "Harbor");
  await click([...container.querySelectorAll("button")].find(item => item.textContent === "Create")!);
  expect(container.textContent).toContain(apiErrorMessage("INVALID_RESPONSE", ""));
  expect(container.textContent).not.toContain("Expected a nonblank title");
});
