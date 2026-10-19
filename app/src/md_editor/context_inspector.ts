import type { AiGenerateRequest } from "./ai_contract";
import {
  assemblePromptSections,
  TOKEN_ESTIMATE_LABEL,
  totalEstimatedTokens,
  promptHistory,
} from "./prompt_sections";

export function buildContextInspector(request: AiGenerateRequest, authorGuidance = "") {
  const sections = assemblePromptSections(request, authorGuidance);
  return {
    sections,
    history: promptHistory(request),
    totalTokens: totalEstimatedTokens(sections),
    tokenEstimateLabel: TOKEN_ESTIMATE_LABEL,
  };
}
