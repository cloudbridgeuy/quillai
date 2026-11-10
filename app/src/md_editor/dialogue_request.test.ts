import { expect, test } from "bun:test";

import { buildDialogueRequest, dialogueSpeakerOptions } from "./dialogue_request";
import type { LoreMatchEntry } from "./lore_match";

type StoredLoreEntry = LoreMatchEntry & { hidden_notes: string };

function entry(id: number, name: string, overrides: Partial<StoredLoreEntry> = {}): StoredLoreEntry {
  return {
    id,
    kind: "character",
    name,
    aliases: [],
    keywords: [],
    visible_text: `${name} text`,
    pinned: false,
    level: "include",
    hidden_notes: `${name} secret`,
    ...overrides,
  };
}

const lore: StoredLoreEntry[] = [
  entry(1, "Zed", { pinned: true }),
  entry(2, "Mara"),
  entry(3, "Ilan", { level: "known_dont_reveal" }),
  entry(4, "Keep", { kind: "place" }),
  entry(5, "Ghost", { level: "leave_out" }),
  entry(6, "Bram"),
  entry(7, "Tower", { kind: "place" }),
];

const text = "Prefix. Mara reached the Keep. Tower";

function build(overrides: Partial<Parameters<typeof buildDialogueRequest>[0]> = {}) {
  return buildDialogueRequest({
    speakers: ["Ilan", "Bram"],
    brief: "  argue about the debt  ",
    loreEntries: lore,
    document: { delta: { ops: [{ insert: text }] }, text },
    window: { start: 100, end: 100 + text.length },
    caret: 130,
    storyContext: " Rainy night ",
    ...overrides,
  });
}

test("speaker options list non-leave_out characters, pins first, then by name", () => {
  expect(dialogueSpeakerOptions(lore).map(option => option.name)).toEqual(["Zed", "Bram", "Ilan", "Mara"]);
});

test("request carries speakers, trimmed brief, story context, and no model or Continue fields", () => {
  const request = build();
  expect(request.action).toBe("dialogue");
  expect(request.dialogue).toEqual({ speakers: ["Ilan", "Bram"], brief: "argue about the debt" });
  expect(request.storyContext).toBe("Rainy night");
  expect(Object.keys(request).sort()).toEqual(["action", "dialogue", "document", "lore", "storyContext"]);
  expect(JSON.parse(JSON.stringify(request))).not.toHaveProperty("model");
});

test("document is cut at the caret so no text after it is sent", () => {
  const request = build();
  expect(request.document).toEqual({
    delta: { ops: [{ insert: "Prefix. Mara reached the Keep." }] },
    text: "Prefix. Mara reached the Keep.",
  });
});

test("speaker lore comes first, then pins and matched entries, without leave_out or hidden notes", () => {
  const request = build();
  expect(request.lore.map(item => item.name)).toEqual(["Ilan", "Bram", "Zed", "Mara", "Keep"]);
  expect(request.lore[0]).toEqual({ name: "Ilan", kind: "character", visible_text: "Ilan text", level: "known_dont_reveal" });
  expect(JSON.stringify(request)).not.toContain("hidden_notes");
  expect(JSON.stringify(request)).not.toContain("secret");
  expect(JSON.stringify(request)).not.toContain("Ghost");
});

test("forced speakers count toward the lore budget", () => {
  const request = build({ maxLoreCharacters: "IlanIlan textBramBram textZedZed text".length + 4 });
  expect(request.lore.map(item => item.name)).toEqual(["Ilan", "Bram", "Zed"]);
});

test("speaker count, duplicates, and unknown or leave_out speakers are rejected", () => {
  expect(() => build({ speakers: [] })).toThrow("1 to 6");
  const seven = Array.from({ length: 7 }, (_, index) => entry(10 + index, `C${index}`));
  expect(() => build({ loreEntries: seven, speakers: seven.map(item => item.name) })).toThrow("1 to 6");
  expect(() => build({ speakers: ["Mara", "Mara"] })).toThrow("duplicate");
  expect(() => build({ speakers: ["Keep"] })).toThrow("Keep");
  expect(() => build({ speakers: ["Ghost"] })).toThrow("Ghost");
  expect(() => build({ speakers: ["Nobody"] })).toThrow("Nobody");
});

test("brief is capped at 4,096 UTF-8 bytes", () => {
  expect(build({ brief: "é".repeat(2_048) }).dialogue.brief).toBe("é".repeat(2_048));
  expect(() => build({ brief: "é".repeat(2_049) })).toThrow("4096");
  expect(build({ brief: undefined }).dialogue.brief).toBe("");
});

test("caret outside the transmitted window is rejected", () => {
  expect(() => build({ caret: 99 })).toThrow("window");
  expect(() => build({ caret: 100 + text.length + 1 })).toThrow("window");
});
