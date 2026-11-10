import { afterEach, expect, mock, test } from "bun:test";
import { act, useState } from "react";

import { cleanup, click, render } from "../../test/dom";
import type { LoreMatchEntry } from "../lore_match";
import type { DialogueChoice } from "../dialogue_stream";
import { DialogueBox } from "./DialogueBox";

afterEach(cleanup);

const entry = (id: number, name: string, extra: Partial<LoreMatchEntry> = {}): LoreMatchEntry => ({
  id, kind: "character", name, aliases: [], keywords: [], visible_text: "", pinned: false, level: "include", ...extra,
});

function Harness({ lore, disabled = false, onWrite }: { lore: LoreMatchEntry[]; disabled?: boolean; onWrite: (choice: DialogueChoice) => void }) {
  const [open, setOpen] = useState(false);
  return <DialogueBox loreEntries={lore} disabled={disabled} open={open} onOpenChange={setOpen} onWrite={onWrite} />;
}

async function openBox(lore: LoreMatchEntry[], options: { disabled?: boolean } = {}) {
  const onWrite = mock((_choice: DialogueChoice) => {});
  await render(<Harness lore={lore} disabled={options.disabled} onWrite={onWrite} />);
  const trigger = document.querySelector<HTMLButtonElement>("button[aria-label='Dialogue']");
  expect(trigger).not.toBeNull();
  await click(trigger!);
  const box = document.querySelector<HTMLElement>("[role='dialog'][aria-label='Dialogue']");
  expect(box).not.toBeNull();
  const speakers = () => [...box!.querySelectorAll<HTMLInputElement>("input[type='checkbox']")];
  const names = () => speakers().map(input => input.closest("label")!.textContent);
  const write = () => [...box!.querySelectorAll("button")].find(button => button.textContent === "Write")!;
  return { box: box!, speakers, names, write, onWrite };
}

async function setBrief(box: HTMLElement, value: string) {
  const brief = box.querySelector<HTMLTextAreaElement>("textarea[aria-label='Dialogue brief']")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(brief, value);
    brief.dispatchEvent(new Event("input", { bubbles: true }));
  });
  return brief;
}

test("the rail button opens a Dialogue box listing characters, pins first, without Leave out", async () => {
  const { names, box } = await openBox([
    entry(1, "Zed"), entry(2, "Ana"), entry(3, "Ghost", { level: "leave_out" }),
    entry(4, "Keep", { kind: "place" }), entry(5, "Mia", { pinned: true }),
  ]);
  expect(names()).toEqual(["Mia", "Ana", "Zed"]);
  expect(box.textContent).not.toContain("Ghost");
});

test("at most six speakers can be chosen", async () => {
  const { speakers } = await openBox(["A", "B", "C", "D", "E", "F", "G"].map((name, index) => entry(index + 1, name)));
  for (const input of speakers().slice(0, 6)) await click(input);
  expect(speakers().map(input => input.checked)).toEqual([true, true, true, true, true, true, false]);
  expect(speakers()[6]!.disabled).toBe(true);
  await click(speakers()[0]!);
  expect(speakers()[6]!.disabled).toBe(false);
});

test("Write sends the chosen speakers and brief, and needs a speaker", async () => {
  const { box, speakers, write, onWrite } = await openBox([entry(1, "Ana"), entry(2, "Bram")]);
  expect(write().disabled).toBe(true);
  await click(speakers()[1]!);
  await click(speakers()[0]!);
  await setBrief(box, "argue about the debt");
  expect(write().disabled).toBe(false);
  await click(write());
  expect(onWrite.mock.calls).toEqual([[{ speakers: ["Bram", "Ana"], brief: "argue about the debt" }]]);
});

test("Enter in the brief writes and Shift+Enter does not", async () => {
  const { box, speakers, onWrite } = await openBox([entry(1, "Ana")]);
  await click(speakers()[0]!);
  const brief = await setBrief(box, "go");
  await act(async () => { brief.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", shiftKey: true, bubbles: true })); });
  expect(onWrite.mock.calls.length).toBe(0);
  await act(async () => { brief.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });
  expect(onWrite.mock.calls).toEqual([[{ speakers: ["Ana"], brief: "go" }]]);
});

test("Write is disabled while the editor is busy or not ready", async () => {
  const { speakers, write } = await openBox([entry(1, "Ana")], { disabled: true });
  await click(speakers()[0]!);
  expect(write().disabled).toBe(true);
});
