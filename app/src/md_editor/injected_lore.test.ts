import { expect, test } from "bun:test";
import { buildAiRequest } from "./ai_contract";
import { buildContextInspector, buildInjectedLore } from "./context_inspector";
import { newLoreDraft } from "../manuscripts/lore_panel";
import { assemblePromptSections } from "./prompt_sections";

const mira = { ...newLoreDraft(), id: 1, series_id: 2, name: "Mira", visible_text: "Navigator 月", hidden_notes: "SECRET" };
const request = buildAiRequest({ action: "generate", document: { text: "Mira arrived.", delta: { ops: [{ insert: "Mira arrived." }] } },
  window: { start: 0, end: 13 }, loreEntries: [mira] });

test("injected lore reports keyword reason, level, type and provider-parity token estimate", () => {
  const view = buildInjectedLore(request, [mira]);
  expect(view.injected).toEqual([{ id: 1, name: "Mira", kind: "character", level: "include", reason: "keyword",
    tokens: assemblePromptSections(request).find(section => section.id === "lore")!.tokens }]);
  expect(view.skipped).toEqual([]);
  expect(JSON.stringify(view)).not.toContain("SECRET");
  expect(JSON.stringify(view)).not.toContain("hidden_notes");
});

test("pins are charged first and may exceed the budget; oversized matches are skipped", () => {
  const pin = { ...mira, id: 2, name: "Elsewhere", pinned: true };
  const view = buildInjectedLore(request, [mira, pin], 0);
  expect(view.injected.map(entry => [entry.id, entry.reason])).toEqual([[2, "pinned"]]);
  expect(view.skipped.map(entry => [entry.id, entry.reason])).toEqual([[1, "keyword"]]);
});

test("budget skips large entries but admits later exact-fit entries, preserving IDs", () => {
  const big = { ...mira, id: 2, visible_text: "x".repeat(100) };
  const view = buildInjectedLore(request, [big, mira], mira.name.length + mira.visible_text.length);
  expect(view.injected.map(entry => entry.id)).toEqual([1]);
  expect(view.skipped.map(entry => entry.id)).toEqual([2]);
});

test("leave-out entries consume no budget; unmatched entries are not budget skips", () => {
  const view = buildInjectedLore(request, [
    { ...mira, id: 2, pinned: true, level: "leave_out" },
    { ...mira, id: 3, name: "Unknown" },
    { ...mira, level: "known_dont_reveal" },
  ]);
  expect(view.injected.map(entry => [entry.id, entry.level])).toEqual([[1, "known_dont_reveal"]]);
  expect(view.skipped).toEqual([]);
});

test("aliases and keywords match only the transmitted window, for Continue and Edit", () => {
  const alias = { ...mira, name: "Captain", aliases: ["Mira"] };
  const keyword = { ...mira, id: 2, name: "Ship", keywords: ["arrived"] };
  for (const action of ["generate", "edit"] as const) {
    expect(buildInjectedLore({ ...request, action }, [alias, keyword]).injected.map(entry => entry.reason)).toEqual(["keyword", "keyword"]);
    expect(buildInjectedLore({ ...request, action, document: { text: "Empty", delta: { ops: [{ insert: "Empty" }] } } }, [alias, keyword]).injected).toEqual([]);
  }
});

test("inspector integrates lore and rejects invalid budgets", () => {
  expect(buildContextInspector(request, "", [mira]).lore.injected).toHaveLength(1);
  expect(buildContextInspector(request).lore.injected).toEqual([]);
  for (const budget of [-1, NaN, 1.5]) expect(() => buildInjectedLore(request, [mira], budget)).toThrow(RangeError);
});
