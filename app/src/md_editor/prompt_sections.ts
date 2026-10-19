import type { AiGenerateRequest, AiHistoryMessage, PromptHistoryMessage } from "./ai_contract";

export type PromptSectionId =
  | "system"
  | "history"
  | "instruction"
  | "story_context"
  | "selection"
  | "document";

export type PromptSection = {
  id: PromptSectionId;
  label: string;
  text: string;
  editableText: string;
  tokens: number;
};

export const TOKEN_ESTIMATE_LABEL = "Estimated tokens (UTF-8 bytes ÷ 4, rounded up per section; excludes chat framing)";

export function estimateTokens(text: string): number {
  return Math.ceil(new TextEncoder().encode(text).length / 4);
}

export function totalEstimatedTokens(sections: readonly PromptSection[]): number {
  return sections.reduce((total, section) => total + section.tokens, 0);
}

export function assemblePromptSections(input: AiGenerateRequest): PromptSection[] {
  const overrides = input.sectionOverrides ?? {};
  const defaultInstruction = input.action === "generate"
    ? optionalText(input.resumePrompt) ?? optionalText(input.instruction)
    : optionalText(input.instruction);
  const instruction = overrides.instruction ?? defaultInstruction ?? "none";
  const storyContext = overrides.story_context ?? optionalText(input.storyContext) ?? "none";
  const selectionText = overrides.selection ?? input.selection?.text ?? "none";
  const selection = input.selection
    ? `index=${input.selection.range.index}, length=${input.selection.range.length}, text=${escapePromptText(selectionText)}`
    : escapePromptText(selectionText);
  const document = overrides.document ?? input.document.text;
  const system = overrides.system ?? "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.";

  return [
    section("system", "System", escapePromptText(system), system),
    section("history", "History", assemblePromptHistory(input).map(({ content }) => content).join(""),
      promptHistory(input).map(({ text }) => text).join("")),
    section("instruction", "Instruction", `You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>${input.action}</action>\n<instruction>\n${escapePromptText(instruction)}\n</instruction>\n`, instruction),
    section("story_context", "Story context", `<story_context>\n${escapePromptText(storyContext)}\n</story_context>\n`, storyContext),
    section("selection", "Selection", `<selection>\n${selection}\n</selection>\n`, selectionText),
    section("document", "Document", `<document>\n${escapePromptText(document)}\n</document>`, document),
  ];
}

export function promptHistory(input: AiGenerateRequest): PromptHistoryMessage[] {
  return input.sectionOverrides?.history ?? (input.action === "generate"
    ? input.messages.map(({ role, content }) => ({ role, text: trimServerText(content) }))
    : []);
}

export function assemblePromptHistory(input: AiGenerateRequest): AiHistoryMessage[] {
  return promptHistory(input).map(({ role, text }) => ({ role, content: escapePromptText(text) }));
}

function section(id: PromptSectionId, label: string, text: string, editableText = text): PromptSection {
  return { id, label, text, editableText, tokens: estimateTokens(text) };
}

function trimServerText(text: string): string {
  return text.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");
}

function optionalText(text: string | null): string | null {
  return text === null ? null : trimServerText(text) || null;
}

function escapePromptText(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}
