import { apiErrorMessage } from "../api_error";
import { defaultLocale, formatMessage } from "../i18n/messages";
import { buildAiGenerateUrl, type LoreCompleteRequest } from "./ai_contract";
import { finalizeAiStreamRemainder, parseAiStreamChunk, type ParseAiStreamChunkResult } from "./ai_stream";

const failed = () => new Error(formatMessage(defaultLocale, "lore.completeFailed"));

export async function streamLoreComplete(request: LoreCompleteRequest, signal: AbortSignal, onText: (text: string) => void): Promise<void> {
  const response = await fetch(buildAiGenerateUrl(), { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(request), signal });
  if (!response.ok) {
    const body: unknown = await response.json().catch(() => null);
    const code = typeof body === "object" && body !== null && "code" in body && typeof body.code === "string" ? body.code : "";
    throw new Error(apiErrorMessage(code, ""));
  }
  if (!response.body) throw failed();
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let remainder = "";
  const finished = (result: ParseAiStreamChunkResult): boolean => {
    remainder = result.remainder;
    if (!result.ok) throw new Error(result.error.message);
    for (const event of result.events) {
      if (event.type === "delta") onText(event.delta.ops.map(op => typeof op.insert === "string" ? op.insert : "").join(""));
      else if (event.type === "error") throw new Error(event.error);
      else if (event.type === "cancelled") throw failed();
      else if (event.type === "done") return true;
    }
    return false;
  };
  for (let chunk = await reader.read(); !chunk.done; chunk = await reader.read()) {
    if (finished(parseAiStreamChunk(decoder.decode(chunk.value, { stream: true }), remainder))) return;
  }
  if (finished(parseAiStreamChunk(decoder.decode(), remainder)) || finished(finalizeAiStreamRemainder(remainder))) return;
  throw failed();
}
