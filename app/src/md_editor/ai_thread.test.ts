import { expect, test } from "bun:test";

import {
  DEFAULT_RESUME_PROMPT,
  finishCancelledThread,
  finishGenerateThread,
  historyForResume,
  initialAiThreadState,
  invalidateThreadForManualEdit,
  resetThreadOnInstructionChange,
  shouldUseResumeThread,
  updateThreadSettings,
} from "./ai_thread";

const base = initialAiThreadState(512);

test("initialAiThreadState keeps sticky default settings", () => {
  expect(base).toEqual({
    thread: null,
    settings: {
      maxOutputTokens: 512,
      resumePrompt: DEFAULT_RESUME_PROMPT,
    },
  });

  expect(updateThreadSettings(base, { maxOutputTokens: 900 }).settings).toEqual({
    maxOutputTokens: 900,
    resumePrompt: DEFAULT_RESUME_PROMPT,
  });
});

test("finishGenerateThread resets on normal finish", () => {
  const state = finishGenerateThread({
    state: base,
    finishReason: "stop",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "output",
    generatedRange: { start: 10, end: 16 },
  });

  expect(state.thread).toBeNull();
});

test("finishCancelledThread resets resumable state", () => {
  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(finishCancelledThread(withThread).thread).toBeNull();
});

test("resetThreadOnInstructionChange resets only when instruction changes", () => {
  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(resetThreadOnInstructionChange(withThread, "continue").thread).not.toBeNull();
  expect(resetThreadOnInstructionChange(withThread, "new instruction").thread).toBeNull();
});

test("finishGenerateThread keeps thread on length finish", () => {
  const state = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(shouldUseResumeThread(state, "continue")).toBe(true);
  expect(historyForResume(state)).toEqual([
    { role: "user", content: "continue" },
    { role: "assistant", content: "partial" },
  ]);
});

test("invalidateThreadForManualEdit resets edits inside generated range", () => {
  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(
    invalidateThreadForManualEdit(withThread, {
      index: 12,
      deletedLength: 0,
      insertedLength: 1,
    }).thread,
  ).toBeNull();
});

test("invalidateThreadForManualEdit shifts generated range for earlier edits", () => {
  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(
    invalidateThreadForManualEdit(withThread, {
      index: 2,
      deletedLength: 0,
      insertedLength: 3,
    }).thread?.generatedRange,
  ).toEqual({ start: 13, end: 20 });
});
