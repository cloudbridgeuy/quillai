import { expect, test } from "bun:test";

import {
  DEFAULT_RESUME_PROMPT,
  finishCancelledThread,
  finishGenerateThread,
  getThreadPresentation,
  historyForResume,
  initialAiThreadState,
  invalidateThreadForManualEdit,
  resetThreadOnInstructionChange,
  shouldUseResumeThread,
  updateThreadSettings,
  threadScopeKey,
  threadScopeKeyFromSearch,
} from "./ai_thread";

const base = initialAiThreadState(512);

test("initialAiThreadState keeps sticky default settings", () => {
  expect(base).toEqual({
    thread: null,
    settings: {
      maxOutputTokens: 512,
      resumePrompt: DEFAULT_RESUME_PROMPT,
      continueModel: null,
      editModel: null,
    },
  });

  expect(updateThreadSettings(base, { maxOutputTokens: 900 }).settings).toEqual({
    maxOutputTokens: 900,
    resumePrompt: DEFAULT_RESUME_PROMPT,
    continueModel: null,
    editModel: null,
  });
});

test("getThreadPresentation describes completion mode without matching history", () => {
  expect(getThreadPresentation(base, "continue", false)).toEqual({
    mode: "completion",
    label: "Continue",
    detail: "No saved reply will be resumed.",
    messageCount: 0,
    canReset: false,
  });

  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(getThreadPresentation(withThread, "changed", false)).toEqual({
    mode: "completion",
    label: "Continue",
    detail: "No saved reply will be resumed.",
    messageCount: 0,
    canReset: true,
  });
});

test("getThreadPresentation describes resume and resuming modes", () => {
  const withThread = finishGenerateThread({
    state: base,
    finishReason: "length",
    instruction: "continue",
    userPrompt: "continue",
    generatedText: "partial",
    generatedRange: { start: 10, end: 17 },
  });

  expect(getThreadPresentation(withThread, "continue", false)).toEqual({
    mode: "resume",
    label: "Continue · from a cut-off reply",
    detail: "2 history messages · range 10–17",
    messageCount: 2,
    canReset: true,
  });

  expect(getThreadPresentation(withThread, "continue", true)).toMatchObject({
    mode: "resuming",
    label: "Continuing from a cut-off reply…",
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

test("thread scope separates chapters and pieces", () => {
  const chapter = threadScopeKey({ seriesId: 1, bookId: 2, chapterId: 3 });
  expect(chapter).not.toBe(threadScopeKey({ seriesId: 1, bookId: 2, chapterId: 4 }));
  expect(chapter).not.toBe(threadScopeKey({ universeId: 1, pieceId: 3 }));
  expect(threadScopeKey({ universeId: 1, pieceId: 3 })).not.toBe(threadScopeKey({ universeId: 1, pieceId: 4 }));
});

test("thread scope rejects malformed, ambiguous, and missing IDs", () => {
  expect(threadScopeKeyFromSearch("?seriesId=1&bookId=2&chapterId=3")).toBe("chapter:1:2:3");
  expect(threadScopeKeyFromSearch("?universeId=1&pieceId=3")).toBe("piece:1:3");
  for (const search of ["", "?seriesId=1&bookId=2", "?seriesId=1&bookId=2&chapterId=0", "?seriesId=1&bookId=2&chapterId=1.5", "?universeId=1&pieceId=NaN", "?universeId=1&pieceId=9007199254740992", "?universeId=1&pieceId=3&pieceId=4", "?universeId=1&pieceId=3&chapterId=3"]) {
    expect(threadScopeKeyFromSearch(search)).toBeNull();
  }
});
