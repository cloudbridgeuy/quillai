import { expect, test } from "bun:test";
import { loreEntryLabel, loreEntryPreview, loreLevel } from "./lore_entry_row";

test("labels lore entries with their kind and pin state", () => {
  const entry = { name: "Mara", kind: "character" as const, pinned: false };
  expect(loreEntryLabel(entry)).toBe("Mara · character");
  expect(loreEntryLabel({ ...entry, pinned: true })).toBe("Mara · character · pinned");
});

test("previews the first non-empty trimmed line", () => {
  expect(loreEntryPreview("  \n \t\n  First line  \nSecond line")).toBe("First line");
  expect(loreEntryPreview("\r\n  First line  \r\nSecond line")).toBe("First line");
  expect(loreEntryPreview(" \t\r\n  ")).toBe("");
  expect(loreEntryPreview("")).toBe("");
});

test("truncates previews after 60 characters", () => {
  const line = "a".repeat(60);
  expect(loreEntryPreview(line)).toBe(line);
  expect(loreEntryPreview(`${line}b`)).toBe(`${line}…`);
});

test("labels lore levels", () => {
  expect(loreLevel("include")).toBe("Include");
  expect(loreLevel("known_dont_reveal")).toBe("Known, don't reveal");
  expect(loreLevel("leave_out")).toBe("Leave out");
  expect(loreLevel("unknown")).toBe("Include");
});
