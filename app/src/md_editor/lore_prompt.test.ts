import { expect, test } from "bun:test";
import { buildAiRequest } from "./ai_contract";
import { buildContextInspector } from "./context_inspector";
import type { LoreMatchEntry } from "./lore_match";

const entry: LoreMatchEntry & { hidden_notes: string } = {
  id: 1, name: "Mara", kind: "character", aliases: ["Queen"], keywords: ["crown"],
  visible_text: "Wields <fire> & ice.", hidden_notes: "PRIVATE AUTHOR NOTES",
  pinned: false, level: "include",
};

for (const action of ["generate", "edit"] as const) {
  test(`${action} matches visible lore, drops leave_out before budget, and marks non-revealed facts`, () => {
    const document = { delta: { ops: [{ insert: "The Queen arrived." }] }, text: "The Queen arrived." };
    const request = buildAiRequest({
      action, document, window: { start: 0, end: document.text.length },
      selection: { ...document, range: { index: 0, length: document.text.length } },
      maxLoreCharacters: 100,
      loreEntries: [
        { ...entry, id: 2, name: "OMITTED", visible_text: "NEVER SENT".repeat(100), level: "leave_out", pinned: true },
        entry,
        { ...entry, id: 3, name: "Secret", visible_text: "Hidden heir", level: "known_dont_reveal", pinned: true },
        { ...entry, id: 4, name: "Unmatched", aliases: [], keywords: [] },
      ],
    });
    expect(request.lore).toEqual([
      { name: "Secret", kind: "character", visible_text: "Hidden heir", level: "known_dont_reveal" },
      { name: "Mara", kind: "character", visible_text: entry.visible_text, level: "include" },
    ]);
    const inspector = buildContextInspector(request);
    const lore = inspector.sections.find(section => section.id === "lore")!;
    expect(lore.text).toContain("<lore>\nName: Secret");
    expect(lore.text).toContain("Known, but MUST NOT be revealed in prose.");
    expect(lore.text).toContain("Wields &lt;fire&gt; &amp; ice.");
    expect(lore.tokens).toBe(Math.ceil(new TextEncoder().encode(lore.text).length / 4));
    for (const forbidden of ["PRIVATE AUTHOR NOTES", "hidden_notes", "NEVER SENT", "OMITTED", "Unmatched", "aliases", "keywords", "reason", "pinned"]) {
      expect(JSON.stringify(request)).not.toContain(forbidden);
      expect(JSON.stringify(inspector)).not.toContain(forbidden);
    }
  });
}

test("no eligible lore preserves the old prompt", () => {
  const document = { delta: { ops: [] }, text: "" };
  const input = { action: "generate" as const, document, window: { start: 0, end: 0 } };
  const baseline = buildContextInspector(buildAiRequest(input));
  expect(buildContextInspector(buildAiRequest({ ...input, loreEntries: [] }))).toEqual(baseline);
  expect(buildContextInspector(buildAiRequest({ ...input, loreEntries: [{ ...entry, pinned: true, level: "leave_out" }] }))).toEqual(baseline);
});
