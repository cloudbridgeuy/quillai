import type { DocumentWindow } from "./window";
import { getApiBaseUrl } from "../api_config";
import type { PromptSectionId } from "./prompt_sections";

export type PromptHistoryMessage = { role: AiHistoryRole; text: string };
export type PromptSectionOverrides = Partial<Record<Exclude<PromptSectionId, "history">, string>> & {
  history?: PromptHistoryMessage[];
};

export type AiAction = "generate" | "edit";

export const DEFAULT_MAX_OUTPUT_TOKENS = 512;
export const MIN_MAX_OUTPUT_TOKENS = 1;
export const MAX_MAX_OUTPUT_TOKENS = 4096;

export type DeltaAttributes = Record<string, string | number | boolean | null>;

export type DeltaInsert = string | Record<string, unknown>;

export type DeltaOperation = {
  insert?: DeltaInsert;
  delete?: number;
  retain?: number;
  attributes?: DeltaAttributes | null;
};

export type DeltaDocument = {
  ops: DeltaOperation[];
};

export type AiSelection = {
  index: number;
  length: number;
};

export type AiDocumentSlice = {
  delta: DeltaDocument;
  text: string;
};

export type AiSelectionSlice = {
  delta: DeltaDocument;
  text: string;
  range: AiSelection;
};

export type AiHistoryRole = "user" | "assistant";

export type AiHistoryMessage = {
  role: AiHistoryRole;
  content: string;
};

type BuildAiRequestBase = {
  sectionOverrides?: PromptSectionOverrides;
  document: AiDocumentSlice;
  window: DocumentWindow;
  storyContext?: string | null;
  instruction?: string | null;
  maxOutputTokens?: number | string | null;
};

export type BuildAiGenerateRequestInput = BuildAiRequestBase & {
  action: "generate";
  selection?: AiSelectionSlice | null;
  messages?: AiHistoryMessage[];
  resumePrompt?: string | null;
};

export type BuildAiEditRequestInput = BuildAiRequestBase & {
  action: "edit";
  selection: AiSelectionSlice;
};

export type BuildAiRequestInput = BuildAiGenerateRequestInput | BuildAiEditRequestInput;

export type AiGenerateRequest = {
  sectionOverrides?: PromptSectionOverrides;
  action: AiAction;
  document: AiDocumentSlice;
  selection: AiSelectionSlice | null;
  storyContext: string | null;
  instruction: string | null;
  messages: AiHistoryMessage[];
  resumePrompt: string | null;
  maxOutputTokens: number;
};

export type AiRequestPreview = {
  request: AiGenerateRequest;
  window: DocumentWindow;
  absoluteSelection: AiSelection | null;
};

export type AiFinishReason = "stop" | "length" | "unknown";

export type AiStreamEvent =
  | { type: "delta"; delta: DeltaDocument }
  | { type: "done"; finishReason: AiFinishReason }
  | { type: "error"; error: string }
  | { type: "cancelled" };

export function buildAiRequest(input: BuildAiRequestInput): AiGenerateRequest {
  const { action, document, window, selection, storyContext, instruction, maxOutputTokens } = input;

  assertTextOnlySlice(document, "document");
  if (selection) {
    assertTextOnlySlice(selection, "selection");
  }

  return {
    ...(input.sectionOverrides && Object.keys(input.sectionOverrides).length > 0
      ? { sectionOverrides: { ...input.sectionOverrides } } : {}),
    action,
    document,
    selection: toRelativeSelection(action, selection, window, document),
    storyContext: normalizeOptionalText(storyContext),
    instruction: normalizeOptionalText(instruction),
    messages: action === "generate" ? normalizeHistoryMessages(input.messages) : [],
    resumePrompt: action === "generate" ? normalizeOptionalText(input.resumePrompt) : null,
    maxOutputTokens: normalizeMaxOutputTokens(maxOutputTokens),
  };
}

export function buildAiRequestPreview(input: BuildAiRequestInput): AiRequestPreview {
  return {
    request: buildAiRequest(input),
    window: input.window,
    absoluteSelection: input.selection ? parseSelectionRange(input.selection.range) : null,
  };
}

function assertTextOnlySlice(
  slice: AiDocumentSlice | AiSelectionSlice,
  kind: "document" | "selection",
): void {
  if (slice.delta.ops.some((op) => typeof op.insert === "object" && op.insert !== null)) {
    throw new Error(`AI ${kind} slices currently support text inserts only`);
  }
}

export function buildAiGenerateUrl(): string {
  return `${getApiBaseUrl()}/ai/generate`;
}

function toRelativeSelection(
  action: AiAction,
  selection: AiSelectionSlice | null | undefined,
  window: DocumentWindow,
  document: AiDocumentSlice,
): AiSelectionSlice | null {
  if (!selection) {
    if (action === "edit") {
      throw new Error("edit requests require a selection");
    }

    return null;
  }

  const range = parseSelectionRange(selection.range);
  const selectionStart = range.index;
  const selectionEnd = range.index + range.length;
  if (selectionStart < window.start || selectionEnd > window.end) {
    throw new Error("selection must be fully contained within the transmitted document window");
  }

  const relativeSelectionStart = selectionStart - window.start;
  const relativeSelectionEnd = relativeSelectionStart + range.length;
  if (relativeSelectionEnd > document.text.length) {
    throw new Error("selection must fit within the transmitted document slice");
  }

  return {
    ...selection,
    range: {
      index: relativeSelectionStart,
      length: range.length,
    },
  };
}

function parseSelectionRange(range: AiSelection): AiSelection {
  const index = parseSelectionRangeValue(range.index, "index");
  const length = parseSelectionRangeValue(range.length, "length");

  return { index, length };
}

function parseSelectionRangeValue(value: number, field: "index" | "length"): number {
  if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0) {
    throw new Error(`selection ${field} must be a finite non-negative integer`);
  }

  return value;
}

function normalizeOptionalText(value?: string | null): string | null {
  if (value == null) {
    return null;
  }

  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}


function normalizeHistoryMessages(messages: AiHistoryMessage[] | undefined): AiHistoryMessage[] {
  return (messages ?? []).map((message) => {
    if (message.role !== "user" && message.role !== "assistant") {
      throw new Error("messages role must be one of: user, assistant");
    }

    const content = normalizeOptionalText(message.content);
    if (content === null) {
      throw new Error("messages content must not be blank");
    }

    return { role: message.role, content };
  });
}

function normalizeMaxOutputTokens(value?: number | string | null): number {
  if (value == null || value === "") {
    return DEFAULT_MAX_OUTPUT_TOKENS;
  }

  const parsed = typeof value === "string" ? Number(value) : value;
  if (!Number.isFinite(parsed) || !Number.isInteger(parsed)) {
    throw new Error("maxOutputTokens must be a finite integer");
  }

  if (parsed < MIN_MAX_OUTPUT_TOKENS) {
    throw new Error(`maxOutputTokens must be at least ${MIN_MAX_OUTPUT_TOKENS}`);
  }

  if (parsed > MAX_MAX_OUTPUT_TOKENS) {
    throw new Error(`maxOutputTokens must be at most ${MAX_MAX_OUTPUT_TOKENS}`);
  }

  return parsed;
}
