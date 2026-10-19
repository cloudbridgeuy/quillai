import type { AiGenerateRequest } from "./ai_contract";

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
  const instruction = input.action === "generate"
    ? optionalText(input.resumePrompt) ?? optionalText(input.instruction)
    : optionalText(input.instruction);
  const selection = input.selection
    ? `index=${input.selection.range.index}, length=${input.selection.range.length}, text=${escapePromptText(input.selection.text)}`
    : "none";

  return [
    section("system", "System", "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only."),
    section("history", "History", input.action === "generate"
      ? input.messages.map((message) => trimServerText(message.content)).join("")
      : ""),
    section("instruction", "Instruction", `You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>${input.action}</action>\n<instruction>\n${escapePromptText(instruction ?? "none")}\n</instruction>\n`),
    section("story_context", "Story context", `<story_context>\n${escapePromptText(optionalText(input.storyContext) ?? "none")}\n</story_context>\n`),
    section("selection", "Selection", `<selection>\n${selection}\n</selection>\n`),
    section("document", "Document", `<document>\n${escapePromptText(input.document.text)}\n</document>`),
  ];
}

function section(id: PromptSectionId, label: string, text: string): PromptSection {
  return { id, label, text, tokens: estimateTokens(text) };
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
