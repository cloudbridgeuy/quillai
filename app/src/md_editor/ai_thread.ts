import type { AiFinishReason, AiHistoryMessage } from "./ai_contract";

export type GeneratedRange = {
  start: number;
  end: number;
};

export type AiThreadSettings = {
  maxOutputTokens: number;
  resumePrompt: string;
};

export type ResumableGenerateThread = {
  kind: "resumable_generate";
  instruction: string | null;
  messages: AiHistoryMessage[];
  generatedRange: GeneratedRange;
};

export type AiThreadState = {
  thread: ResumableGenerateThread | null;
  settings: AiThreadSettings;
};

export type ManualEdit = {
  index: number;
  deletedLength: number;
  insertedLength: number;
};

export const DEFAULT_RESUME_PROMPT = "Continue exactly where the previous output stopped.";

export function initialAiThreadState(maxOutputTokens: number): AiThreadState {
  return {
    thread: null,
    settings: {
      maxOutputTokens,
      resumePrompt: DEFAULT_RESUME_PROMPT,
    },
  };
}

export function updateThreadSettings(
  state: AiThreadState,
  patch: Partial<AiThreadSettings>,
): AiThreadState {
  return {
    ...state,
    settings: {
      ...state.settings,
      ...patch,
    },
  };
}

export function historyForResume(state: AiThreadState): AiHistoryMessage[] {
  return state.thread?.messages ?? [];
}

export function shouldUseResumeThread(
  state: AiThreadState,
  instruction: string | null,
): boolean {
  return state.thread !== null && state.thread.instruction === instruction;
}

export function resetThread(state: AiThreadState): AiThreadState {
  return { ...state, thread: null };
}

export function finishGenerateThread(input: {
  state: AiThreadState;
  finishReason: AiFinishReason;
  instruction: string | null;
  userPrompt: string | null;
  generatedText: string;
  generatedRange: GeneratedRange;
}): AiThreadState {
  if (input.finishReason !== "length" || input.generatedText.length === 0) {
    return resetThread(input.state);
  }

  return {
    ...input.state,
    thread: {
      kind: "resumable_generate",
      instruction: input.instruction,
      generatedRange: input.generatedRange,
      messages: [
        ...historyForResume(input.state),
        { role: "user", content: input.userPrompt ?? "Continue the scene." },
        { role: "assistant", content: input.generatedText },
      ],
    },
  };
}

export function finishCancelledThread(state: AiThreadState): AiThreadState {
  return resetThread(state);
}

export function resetThreadOnInstructionChange(
  state: AiThreadState,
  nextInstruction: string | null,
): AiThreadState {
  if (state.thread === null || state.thread.instruction === nextInstruction) {
    return state;
  }

  return resetThread(state);
}

export function manualEditFromDelta(delta: { ops?: Array<{ insert?: unknown; delete?: number; retain?: unknown }> }): ManualEdit {
  let index = 0;
  let deletedLength = 0;
  let insertedLength = 0;

  for (const op of delta.ops ?? []) {
    if (typeof op.retain === "number") {
      index += op.retain;
      continue;
    }

    if (typeof op.delete === "number") {
      deletedLength += op.delete;
      continue;
    }

    if (typeof op.insert === "string") {
      insertedLength += op.insert.length;
    } else if (op.insert !== undefined) {
      insertedLength += 1;
    }
  }

  return { index, deletedLength, insertedLength };
}

export function invalidateThreadForManualEdit(
  state: AiThreadState,
  edit: ManualEdit,
): AiThreadState {
  const thread = state.thread;
  if (!thread) {
    return state;
  }

  const editEnd = edit.index + edit.deletedLength;
  const touchesGeneratedRange = edit.index <= thread.generatedRange.end && editEnd >= thread.generatedRange.start;
  if (touchesGeneratedRange) {
    return resetThread(state);
  }

  const shift = edit.insertedLength - edit.deletedLength;
  if (edit.index >= thread.generatedRange.end || shift === 0) {
    return state;
  }

  return {
    ...state,
    thread: {
      ...thread,
      generatedRange: {
        start: Math.max(0, thread.generatedRange.start + shift),
        end: Math.max(0, thread.generatedRange.end + shift),
      },
    },
  };
}
