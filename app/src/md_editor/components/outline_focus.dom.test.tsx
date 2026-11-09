import { afterEach, expect, test } from "bun:test";
import { act, useRef, useState } from "react";
import { cleanup, render } from "../../test/dom";
import { useRestoreOutlineFocus } from "./outline_focus";

afterEach(cleanup);

let setPending!: (value: boolean) => void;
function Harness() {
  const ref = useRef<HTMLDivElement>(null);
  const [pending, set] = useState(false);
  setPending = set;
  useRestoreOutlineFocus(ref, pending);
  return <div><div ref={ref} tabIndex={0} id="outline"><button aria-pressed="true" id="row">Chapter</button><button id="row2">Two</button><button id="gone">Gone</button></div><textarea id="other" /></div>;
}

async function switchWith(focusId: string, after?: string) {
  const view = await render(<Harness />);
  view.querySelector<HTMLElement>(`#${focusId}`)!.focus();
  await act(async () => setPending(true));
  (document.activeElement as HTMLElement).blur();
  if (after) view.querySelector<HTMLElement>(`#${after}`)!.focus();
  await act(async () => setPending(false));
  return view;
}

test("returns focus to the region when the region held it", async () => {
  await switchWith("outline");
  expect(document.activeElement?.id).toBe("outline");
});

test("returns focus to the previously focused row when still connected", async () => {
  await switchWith("row2");
  expect(document.activeElement?.id).toBe("row2");
});

test("falls back to the pressed row when the previous element was removed", async () => {
  const view = await render(<Harness />);
  view.querySelector<HTMLElement>("#gone")!.focus();
  await act(async () => setPending(true));
  (document.activeElement as HTMLElement).blur();
  view.querySelector("#gone")!.remove();
  await act(async () => setPending(false));
  expect(document.activeElement?.id).toBe("row");
});

test("leaves focus alone when it started outside or moved elsewhere", async () => {
  await switchWith("other");
  expect(document.activeElement).toBe(document.body);
  await cleanup();
  await switchWith("outline", "other");
  expect(document.activeElement?.id).toBe("other");
});

test("returns focus to the region when a removed draft field held it", async () => {
  const view = await render(<Harness />);
  const draft = document.createElement("input");
  view.querySelector("#outline")!.append(draft);
  draft.focus();
  await act(async () => setPending(true));
  draft.blur();
  draft.remove();
  await act(async () => setPending(false));
  expect(document.activeElement?.id).toBe("outline");
});
