import { DEFAULT_MAX_LORE_CHARACTERS, type AiGenerateRequest } from "./ai_contract";
import { matchLoreEntries, matchLoreOccurrences, type LoreMatchEntry } from "./lore_match";
import {
  assemblePromptSections,
  TOKEN_ESTIMATE_LABEL,
  totalEstimatedTokens,
  promptHistory,
} from "./prompt_sections";

export function buildInjectedLore(request: AiGenerateRequest, entries: readonly LoreMatchEntry[], maxCharacters = DEFAULT_MAX_LORE_CHARACTERS) {
  const occurrences = matchLoreOccurrences(request.document.text, entries);
  const matches = matchLoreEntries(request.document.text, entries, Number.MAX_SAFE_INTEGER).map(entry => ({
    ...entry, occurrences: occurrences.filter(match => match.id === entry.id),
  }));
  const eligible = entries.filter(entry => entry.level !== "leave_out");
  const injected = matchLoreEntries(request.document.text, eligible, maxCharacters);
  const ids = new Set(injected.map(entry => entry.id));
  const skipped = matchLoreEntries(request.document.text, eligible, Number.MAX_SAFE_INTEGER)
    .filter(entry => !ids.has(entry.id));
  function row(entry: (typeof injected)[number]) {
    const { id, name, kind, level, reason, visible_text } = entry;
    const tokens = level === "leave_out" ? 0 : assemblePromptSections({
      ...request, lore: [{ name, kind, visible_text, level }],
    }).find(section => section.id === "lore")?.tokens ?? 0;
    return { id, name, kind, level, reason, tokens, visible_text, occurrences: occurrences.filter(match => match.id === id) };
  }
  return { injected: injected.map(row), skipped: skipped.map(row), matches, maxCharacters };
}

export function buildContextInspector(request: AiGenerateRequest, authorGuidance = "", loreEntries: readonly LoreMatchEntry[] = []) {
  const sections = assemblePromptSections(request, authorGuidance);
  const history = promptHistory(request);
  return {
    sections,
    history,
    requestLabel: request.action === "edit" ? "Edit selection" : history.length > 0 ? "Continue · with thread" : "Continue",
    totalTokens: totalEstimatedTokens(sections),
    tokenEstimateLabel: TOKEN_ESTIMATE_LABEL,
    lore: buildInjectedLore(request, loreEntries),
  };
}
