import { afterEach, expect, test } from "bun:test";
import Quill from "quill";
import { Delta } from "quill/core";
import { act } from "react";
import { defaultLocale, formatMessage } from "../../i18n/messages";
import { cleanup, render } from "../../test/dom";
import type { Hint, Rule } from "./engine";
import { HINT_TIP_DELAY_MS, HintTip } from "./HintTip";

afterEach(cleanup);

const rule: Rule = { id: "cat", labelKey: "rules.adverbs.label", languages: ["en"], defaultEnabled: true, params: [], defaultScope: "both", check: () => [] };
const hint: Hint = { ruleId: "cat", start: 6, length: 4, messageKey: "hints.adverb" };
const label = formatMessage(defaultLocale, "rules.adverbs.label");
const message = formatMessage(defaultLocale, "hints.adverb");

async function setup(hints: readonly Hint[] = [hint]) {
  const quill = new Quill(document.body.appendChild(document.createElement("div")));
  quill.setContents(new Delta().insert("alpha beta gamma\n"), "silent");
  quill.getBounds = (index: number, length = 0) => ({ left: index * 10, top: 10, width: length * 10, height: 20, bottom: 30, right: index * 10 + length * 10 });
  let now = 0;
  const pending = new Map<number, { at: number; run: () => void }>();
  let nextId = 1;
  const timers = {
    set: (run: () => void, ms: number) => { const id = nextId++; pending.set(id, { at: now + ms, run }); return id; },
    clear: (handle: unknown) => { pending.delete(handle as number); },
  };
  const advance = (ms: number) => act(async () => {
    now += ms;
    for (const [id, timer] of [...pending]) if (timer.at <= now) { pending.delete(id); timer.run(); }
  });
  await render(<HintTip quill={quill} hints={hints} rules={[rule]} timers={timers} />);
  const tip = () => document.body.querySelector<HTMLElement>("[role=tooltip]");
  const fire = (type: string, x: number, y: number, target: Element = quill.root) => act(async () => { target.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: x, clientY: y })); });
  const tap = async (x: number, y: number) => { await fire("pointerdown", x, y); await fire("click", x, y); };
  const escape = () => act(async () => { document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
  return { quill, advance, tip, fire, tap, escape };
}

const onHint = [70, 20] as const;

test("the pointer resting on a hint opens the tip with the rule name and message", async () => {
  const { advance, tip, fire } = await setup();
  await fire("pointermove", ...onHint);
  await advance(HINT_TIP_DELAY_MS - 1);
  expect(tip()).toBeNull();
  await advance(1);
  expect(tip()?.textContent ?? "").toContain(label);
  expect(tip()?.textContent ?? "").toContain(message);
});

test("moving the pointer restarts the rest delay", async () => {
  const { advance, tip, fire } = await setup();
  await fire("pointermove", ...onHint);
  await advance(HINT_TIP_DELAY_MS - 1);
  await fire("pointermove", 72, 20);
  await advance(HINT_TIP_DELAY_MS - 1);
  expect(tip()).toBeNull();
  await advance(1);
  expect(tip()).not.toBeNull();
});

test("a pointer resting away from every hint opens nothing", async () => {
  const { advance, tip, fire } = await setup();
  await fire("pointermove", 500, 20);
  await advance(HINT_TIP_DELAY_MS * 2);
  expect(tip()).toBeNull();
});

test("a caret inside a hint opens the tip at once and leaving it closes the tip", async () => {
  const { quill, tip } = await setup();
  await act(async () => quill.setSelection(8, 0, "user"));
  expect(tip()?.textContent ?? "").toContain(message);
  await act(async () => quill.setSelection(1, 0, "user"));
  expect(tip()).toBeNull();
});

test("a selection range does not open the tip", async () => {
  const { quill, tip } = await setup();
  await act(async () => quill.setSelection(6, 4, "user"));
  expect(tip()).toBeNull();
});

test("Escape closes the tip", async () => {
  const { quill, tip, escape } = await setup();
  await act(async () => quill.setSelection(8, 0, "user"));
  expect(tip()).not.toBeNull();
  await escape();
  expect(tip()).toBeNull();
});

test("a tap toggles the tip", async () => {
  const { tip, tap } = await setup();
  await tap(...onHint);
  expect(tip()).not.toBeNull();
  await tap(...onHint);
  expect(tip()).toBeNull();
});

test("opening, tapping and closing leave the document, selection and focus alone", async () => {
  const { quill, advance, tip, fire, tap, escape } = await setup();
  await act(async () => quill.setSelection(2, 0, "user"));
  const contents = JSON.stringify(quill.getContents());
  const html = quill.root.innerHTML;
  const focused = document.activeElement;
  await fire("pointermove", ...onHint);
  await advance(HINT_TIP_DELAY_MS);
  expect(tip()).not.toBeNull();
  const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
  await act(async () => { tip()!.dispatchEvent(down); });
  expect(down.defaultPrevented).toBe(true);
  await tap(...onHint);
  await escape();
  expect(tip()).toBeNull();
  expect(JSON.stringify(quill.getContents())).toBe(contents);
  expect(quill.root.innerHTML).toBe(html);
  expect(quill.getSelection()).toEqual({ index: 2, length: 0 });
  expect(document.activeElement).toBe(focused);
});
