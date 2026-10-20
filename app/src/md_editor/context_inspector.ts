import type { AiGenerateRequest } from "./ai_contract";
import {
  assemblePromptSections,
  TOKEN_ESTIMATE_LABEL,
  totalEstimatedTokens,
  promptHistory,
} from "./prompt_sections";

export function buildContextInspector(request: AiGenerateRequest, authorGuidance = "") {
  const sections = assemblePromptSections(request, authorGuidance);
  const history = promptHistory(request);
  return {
    sections,
    history,
    requestLabel: request.action === "edit" ? "Edit selection" : history.length > 0 ? "Continue · with thread" : "Continue",
    totalTokens: totalEstimatedTokens(sections),
    tokenEstimateLabel: TOKEN_ESTIMATE_LABEL,
  };
}
