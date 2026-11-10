import { afterEach, expect, test } from "bun:test";
import Quill from "quill";
import { Delta } from "quill/core";
import { act } from "react";
import { cleanup, render } from "../../test/dom";
import type { Hint } from "./engine";
import { HintOverlay } from "./HintOverlay";

afterEach(cleanup);

const hint = (start: number, length: number): Hint => ({ ruleId: "cat", start, length, messageKey: "hints.adverb" });

function setup(hints: Hint[]) {
  const quill = new Quill(document.body.appendChild(document.createElement("div")));
  quill.setContents(new Delta().insert("alpha beta gamma\n"), "silent");
  let top = 10;
  quill.getBounds = (index: number, length = 0) => ({ left: index * 10, top, width: length * 10, height: 20, bottom: top + 20, right: index * 10 + length * 10 });
  return { quill, moveTo: (next: number) => { top = next; }, mount: () => render(<HintOverlay quill={quill} hints={hints} />) };
}

const underlines = (quill: Quill) => [...quill.container.querySelectorAll<HTMLElement>("[data-hint-underline]")];

test("draws one underline per hint line inside the editor container", async () => {
  const { quill, mount } = setup([hint(6, 4), hint(11, 5)]);
  await mount();
  expect(underlines(quill).map((el) => [el.style.left, el.style.top, el.style.width, el.style.height])).toEqual([
    ["60px", "10px", "40px", "20px"],
    ["110px", "10px", "50px", "20px"],
  ]);
});

test("underlines are inert, hidden from assistive tech, and layered by the hint token", async () => {
  const { quill, mount } = setup([hint(6, 4)]);
  await mount();
  const [el] = underlines(quill);
  expect(el?.getAttribute("aria-hidden")).toBe("true");
  expect(el?.className).toContain("pointer-events-none");
  expect(el?.style.zIndex).toBe("var(--z-hint)");
});

test("repositions on window resize and on scroll", async () => {
  const { quill, moveTo, mount } = setup([hint(6, 4)]);
  await mount();
  moveTo(50);
  await act(async () => { window.dispatchEvent(new Event("resize")); });
  expect(underlines(quill)[0]?.style.top).toBe("50px");
  moveTo(90);
  await act(async () => { window.dispatchEvent(new Event("scroll")); });
  expect(underlines(quill)[0]?.style.top).toBe("90px");
});

test("removes its window listeners on unmount", async () => {
  const live: unknown[] = [];
  const add = window.addEventListener.bind(window);
  const remove = window.removeEventListener.bind(window);
  window.addEventListener = (type: string, handler: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions) => { if (type === "resize" || type === "scroll") live.push(handler); add(type, handler, options); };
  window.removeEventListener = (type: string, handler: EventListenerOrEventListenerObject, options?: boolean | EventListenerOptions) => { live.splice(live.indexOf(handler), 1); remove(type, handler, options); };
  try {
    const { mount } = setup([hint(6, 4)]);
    await mount();
    expect(live.length).toBe(2);
    await cleanup();
    expect(live.length).toBe(0);
  } finally {
    window.addEventListener = add;
    window.removeEventListener = remove;
  }
});

test("drawing hints leaves the Quill document untouched", async () => {
  const { quill, mount } = setup([hint(6, 4), hint(11, 5)]);
  const contents = JSON.stringify(quill.getContents());
  const html = quill.root.innerHTML;
  await mount();
  expect(underlines(quill)).toHaveLength(2);
  expect(JSON.stringify(quill.getContents())).toBe(contents);
  expect(quill.root.innerHTML).toBe(html);
});

test("no hints draws nothing", async () => {
  const { quill, mount } = setup([]);
  await mount();
  expect(underlines(quill)).toEqual([]);
});

test("two hints on the same word both draw, with unique React keys", async () => {
  const { quill, moveTo, mount } = setup([hint(6, 4), { ...hint(6, 4), ruleId: "other" }]);
  const errors: string[] = [];
  const original = console.error;
  console.error = (...args: unknown[]) => { errors.push(args.map(String).join(" ")); };
  try {
    await mount();
    expect(underlines(quill)).toHaveLength(2);
    moveTo(50);
    await act(async () => { window.dispatchEvent(new Event("resize")); });
    expect(underlines(quill).map((el) => el.style.top)).toEqual(["50px", "50px"]);
  } finally {
    console.error = original;
  }
  expect(errors.filter((line) => line.includes("same key"))).toEqual([]);
});
