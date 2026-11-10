import Delta from "quill-delta";

import type { AiDocumentSlice, AiLoreEntry } from "./ai_contract";
import { DEFAULT_MAX_LORE_CHARACTERS } from "./ai_contract";
import { toContractDeltaDocument, toQuillDeltaDocument } from "./delta_bridge";
import { matchLoreEntries, type LoreMatchEntry } from "./lore_match";
import type { DocumentWindow } from "./window";

export const MAX_DIALOGUE_SPEAKERS = 6;
export const MAX_DIALOGUE_BRIEF_BYTES = 4_096;

export type DialogueRequest = {
  action: "dialogue";
  dialogue: { speakers: string[]; brief: string };
  document: AiDocumentSlice;
  storyContext: string | null;
  lore: AiLoreEntry[];
};

export type BuildDialogueRequestInput = {
  speakers: readonly string[];
  brief?: string;
  loreEntries: readonly LoreMatchEntry[];
  maxLoreCharacters?: number;
  document: AiDocumentSlice;
  window: DocumentWindow;
  caret: number;
  storyContext?: string | null;
};

export function dialogueSpeakerOptions(entries: readonly LoreMatchEntry[]): LoreMatchEntry[] {
  return entries
    .filter(entry => entry.kind === "character" && entry.level !== "leave_out")
    .sort((a, b) => Number(b.pinned) - Number(a.pinned) || a.name.localeCompare(b.name));
}

export function buildDialogueRequest(input: BuildDialogueRequestInput): DialogueRequest {
  const { speakers, loreEntries, window, caret } = input;
  if (speakers.length < 1 || speakers.length > MAX_DIALOGUE_SPEAKERS) {
    throw new Error(`dialogue speakers must name 1 to ${MAX_DIALOGUE_SPEAKERS} characters`);
  }
  const options = dialogueSpeakerOptions(loreEntries);
  const speakerEntries = speakers.map((speaker, index) => {
    if (speakers.indexOf(speaker) !== index) throw new Error(`dialogue speakers has a duplicate: ${speaker}`);
    const found = options.find(option => option.name === speaker);
    if (!found) throw new Error(`dialogue speaker ${speaker} is not a character in lore`);
    return found;
  });

  const brief = (input.brief ?? "").trim();
  if (new TextEncoder().encode(brief).length > MAX_DIALOGUE_BRIEF_BYTES) {
    throw new Error(`dialogue brief exceeds ${MAX_DIALOGUE_BRIEF_BYTES} UTF-8 bytes`);
  }

  if (!Number.isInteger(caret) || caret < window.start || caret > window.end) {
    throw new Error("dialogue caret must be within the transmitted document window");
  }
  const cut = caret - window.start;
  const document = { delta: toContractDeltaDocument(new Delta(toQuillDeltaDocument(input.document.delta)).slice(0, cut)), text: input.document.text.slice(0, cut) };

  const candidates = [
    ...speakerEntries.map(entry => ({ ...entry, pinned: true })),
    ...loreEntries.filter(entry => entry.level !== "leave_out" && !speakerEntries.includes(entry)),
  ];
  const lore = matchLoreEntries(document.text, candidates, input.maxLoreCharacters ?? DEFAULT_MAX_LORE_CHARACTERS)
    .flatMap(({ name, kind, visible_text, level }) => (level === "leave_out" ? [] : [{ name, kind, visible_text, level }]));

  const storyContext = input.storyContext?.trim() || null;
  return { action: "dialogue", dialogue: { speakers: [...speakers], brief }, document, storyContext, lore };
}
