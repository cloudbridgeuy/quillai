import { DEFAULT_MAX_OUTPUT_TOKENS } from "./ai_contract";
import type { AiHistoryMessage, DeltaOperation } from "./ai_contract";
import {
  initialAiThreadState,
  type AiThreadState,
  type ResumableGenerateThread,
} from "./ai_thread";
import type { DocumentWindowMode } from "./window";

export const PERSISTED_EDITOR_STATE_KEY = "quillai.editor.state.v1";

export type PersistedDeltaDocument = {
  ops: DeltaOperation[];
};

export type PersistedEditorState = {
  document: PersistedDeltaDocument;
  storyContext: string;
  instruction: string;
  windowSize: string;
  windowMode: DocumentWindowMode;
  threadState: AiThreadState;
};

export function initialPersistedEditorState(): PersistedEditorState {
  return {
    document: { ops: [] },
    storyContext: "",
    instruction: "",
    windowSize: "1200",
    windowMode: "window",
    threadState: initialAiThreadState(DEFAULT_MAX_OUTPUT_TOKENS),
  };
}

export function parsePersistedEditorState(raw: string | null): PersistedEditorState {
  const fallback = initialPersistedEditorState();
  if (raw === null) {
    return fallback;
  }

  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!isRecord(parsed)) {
      return fallback;
    }

    return {
      document: parseDeltaDocument(parsed.document, fallback.document),
      storyContext: typeof parsed.storyContext === "string" ? parsed.storyContext : fallback.storyContext,
      instruction: typeof parsed.instruction === "string" ? parsed.instruction : fallback.instruction,
      windowSize: typeof parsed.windowSize === "string" ? parsed.windowSize : fallback.windowSize,
      windowMode: parseWindowMode(parsed.windowMode, fallback.windowMode),
      threadState: parseThreadState(parsed.threadState, fallback.threadState),
    };
  } catch {
    return fallback;
  }
}

export function serializePersistedEditorState(state: PersistedEditorState): string {
  return JSON.stringify(state);
}

function parseDeltaDocument(
  value: unknown,
  fallback: PersistedDeltaDocument,
): PersistedDeltaDocument {
  if (!isRecord(value) || !Array.isArray(value.ops)) {
    return fallback;
  }

  const ops: DeltaOperation[] = [];
  for (const op of value.ops) {
    if (!isDeltaOperation(op)) {
      return fallback;
    }
    ops.push(op);
  }

  return { ops };
}

function parseWindowMode(value: unknown, fallback: DocumentWindowMode): DocumentWindowMode {
  return value === "document" || value === "window" ? value : fallback;
}

function parseThreadState(value: unknown, fallback: AiThreadState): AiThreadState {
  if (!isRecord(value) || !isRecord(value.settings)) {
    return fallback;
  }

  const maxOutputTokens = value.settings.maxOutputTokens;
  const resumePrompt = value.settings.resumePrompt;
  if (typeof maxOutputTokens !== "number" || typeof resumePrompt !== "string") {
    return fallback;
  }

  const thread = parseThread(value.thread);
  if (thread === undefined) {
    return fallback;
  }

  return {
    thread,
    settings: { maxOutputTokens, resumePrompt },
  };
}

function parseThread(value: unknown): ResumableGenerateThread | null | undefined {
  if (value === null) {
    return null;
  }

  if (!isRecord(value)) {
    return undefined;
  }

  if (value.kind !== "resumable_generate") {
    return undefined;
  }

  if (value.instruction !== null && typeof value.instruction !== "string") {
    return undefined;
  }

  if (!Array.isArray(value.messages) || !isGeneratedRange(value.generatedRange)) {
    return undefined;
  }

  const messages: AiHistoryMessage[] = [];
  for (const valueMessage of value.messages) {
    const message = parseHistoryMessage(valueMessage);
    if (message === null) return undefined;
    messages.push(message);
  }

  return {
    kind: "resumable_generate",
    instruction: value.instruction,
    messages,
    generatedRange: value.generatedRange,
  };
}

function parseHistoryMessage(value: unknown): AiHistoryMessage | null {
  if (!isRecord(value)) {
    return null;
  }

  if ((value.role !== "user" && value.role !== "assistant") || typeof value.content !== "string") {
    return null;
  }

  return { role: value.role, content: value.content };
}

function isGeneratedRange(value: unknown): value is { start: number; end: number } {
  return (
    isRecord(value) &&
    typeof value.start === "number" &&
    typeof value.end === "number" &&
    Number.isFinite(value.start) &&
    Number.isFinite(value.end)
  );
}

function isDeltaOperation(value: unknown): value is DeltaOperation {
  if (!isRecord(value)) {
    return false;
  }

  if (
    value.insert !== undefined &&
    typeof value.insert !== "string" &&
    !isRecord(value.insert)
  ) {
    return false;
  }

  if (value.delete !== undefined && typeof value.delete !== "number") {
    return false;
  }

  if (value.retain !== undefined && typeof value.retain !== "number") {
    return false;
  }

  return value.attributes === undefined || value.attributes === null || isRecord(value.attributes);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
