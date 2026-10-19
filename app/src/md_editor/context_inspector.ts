import type { AiGenerateRequest } from "./ai_contract";
import {
  assemblePromptSections,
  TOKEN_ESTIMATE_LABEL,
  totalEstimatedTokens,
} from "./prompt_sections";

export function buildContextInspector(request: AiGenerateRequest) {
  const sections = assemblePromptSections(request);
  return {
    sections,
    totalTokens: totalEstimatedTokens(sections),
    tokenEstimateLabel: TOKEN_ESTIMATE_LABEL,
  };
}
