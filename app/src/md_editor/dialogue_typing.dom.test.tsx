import { afterEach, expect, test } from "bun:test";
import { createRef } from "react";
import type Quill from "quill";
import { Delta } from "quill/core";
import { cleanup, render } from "../test/dom";
import { Component } from "./components/Editor";

afterEach(cleanup);

async function mount(options: { markdownShortcuts?: boolean; dashesBlocked?: () => boolean } = {}): Promise<Quill> {
  const ref = createRef<Quill>();
  await render(
    <Component
      ref={ref}
      readOnly={false}
      defaultValue={new Delta()}
      onTextChange={() => {}}
      onSelectionChange={() => {}}
      markdownShortcuts={options.markdownShortcuts ?? true}
      dashesBlocked={options.dashesBlocked}
    />,
  );
  return ref.current!;
}

const text = (quill: Quill) => quill.getText().slice(0, -1);

function type(quill: Quill, keys: string) {
  for (const key of keys) quill.updateContents(new Delta().retain(quill.getLength() - 1).insert(key), "user");
}

test("typed quotes and dashes convert by the line context", async () => {
  const quill = await mount();
  type(quill, "\"Hi,\" she said. don't \"She said 'no'\" --Hola --dijo--. --- rock 'n' roll");
  expect(text(quill)).toBe("“Hi,” she said. don’t “She said ‘no’” —Hola —dijo—. —- rock ‘n’ roll");
});

test("a quote typed over a selection converts", async () => {
  const quill = await mount();
  type(quill, "say xx");
  quill.updateContents(new Delta().retain(4).insert("\"").delete(2), "user");
  expect(text(quill)).toBe("say “");
});

test("paste, AI text, and other sources stay straight", async () => {
  const quill = await mount();
  quill.updateContents(new Delta().insert("\"x\" -- 'y'"), "user");
  quill.updateContents(new Delta().retain(quill.getLength() - 1).insert(" \"").insert("-"), "api");
  type(quill, " ");
  quill.updateContents(new Delta().retain(quill.getLength() - 1).insert("\""), "api");
  quill.updateContents(new Delta().retain(quill.getLength() - 1).insert("'"), "silent");
  expect(text(quill)).toBe("\"x\" -- 'y' \"- \"'");
});

test("markdown shortcuts off turns dialogue typing off", async () => {
  const quill = await mount({ markdownShortcuts: false });
  type(quill, "\"a\" -- 'b'");
  expect(text(quill)).toBe("\"a\" -- 'b'");
});

test("one undo restores the straight quote, a second removes it, redo reapplies the curly form", async () => {
  const quill = await mount();
  type(quill, "\"");
  expect(text(quill)).toBe("“");
  quill.history.undo();
  expect(text(quill)).toBe("\"");
  quill.history.undo();
  expect(text(quill)).toBe("");
  quill.history.redo();
  expect(text(quill)).toBe("\"");
  quill.history.redo();
  expect(text(quill)).toBe("“");
});

test("an active Continue stream blocks only the dash rule", async () => {
  let streaming = true;
  const quill = await mount({ dashesBlocked: () => streaming });
  type(quill, "--\"");
  expect(text(quill)).toBe("--”");
  streaming = false;
  type(quill, " --");
  expect(text(quill)).toBe("--” —");
});

test("a digit after a rule-made opening quote turns it into an apostrophe", async () => {
  const quill = await mount();
  type(quill, "the '90s");
  expect(text(quill)).toBe("the ’90s");
});

test("hand-typed and pasted opening quotes never fix up", async () => {
  const quill = await mount();
  type(quill, "the ‘9 ");
  quill.updateContents(new Delta().retain(quill.getLength() - 1).insert("x ‘"), "user");
  type(quill, "9");
  expect(text(quill)).toBe("the ‘9 x ‘9");
});

test("another keystroke after the rule-made quote clears the fix-up", async () => {
  const quill = await mount();
  type(quill, "'a");
  quill.updateContents(new Delta().retain(1).delete(1), "user");
  type(quill, "9");
  expect(text(quill)).toBe("‘9");
});

test("undoing the rule-made quote clears the fix-up", async () => {
  const quill = await mount();
  type(quill, "'");
  quill.history.undo();
  quill.history.redo();
  expect(text(quill)).toBe("‘");
  type(quill, "9");
  expect(text(quill)).toBe("‘9");
});

test("no conversion while Quill reports an active composition", async () => {
  const quill = await mount();
  type(quill, "a ");
  quill.composition.isComposing = true;
  type(quill, "\"");
  quill.composition.isComposing = false;
  expect(text(quill)).toBe("a \"");
});

async function compose(quill: Quill, committed: string) {
  const line = quill.root.querySelector("p")!;
  line.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true }));
  (line.firstChild as Text).data += committed;
  line.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: committed }));
  for (let tick = 0; tick < 5; tick++) await Promise.resolve();
}

test("a composition that commits exactly one quote converts", async () => {
  const quill = await mount();
  type(quill, "a ");
  await compose(quill, "\"");
  expect(quill.composition.isComposing).toBe(false);
  expect(text(quill)).toBe("a “");
});

test("a composition that commits more than one character stays", async () => {
  const quill = await mount();
  type(quill, "a ");
  await compose(quill, "\"x");
  expect(quill.composition.isComposing).toBe(false);
  expect(text(quill)).toBe("a \"x");
});
