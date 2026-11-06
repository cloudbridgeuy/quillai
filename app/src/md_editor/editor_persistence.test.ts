import { expect, test } from "bun:test";

import { DEFAULT_RESUME_PROMPT } from "./ai_thread";
import {
  initialPersistedEditorState,
  parsePersistedEditorState,
  serializePersistedEditorState,
  type PersistedEditorState,
} from "./editor_persistence";

const persistedState: PersistedEditorState = {
  document: {
    ops: [
      { insert: "Chapter One\n", attributes: { header: 1 } },
      { insert: "A line of text.\n" },
    ],
  },
  storyContext: "Gothic mystery in close third.",
  editInstruction: "Tighten the selected prose.",
  continueInstruction: "Continue with restrained dread.",
  windowSize: "2400",
  windowMode: "document",
  aiTextTint: false,
  showContinueWindow: false,
  threadState: {
    settings: {
      maxOutputTokens: 900,
      resumePrompt: "Keep going from the last sentence.",
    },
    thread: null,
  },
  threads: {
    "chapter:1:2:3": {
      kind: "resumable_generate",
      instruction: "Continue with restrained dread.",
      generatedRange: { start: 12, end: 32 },
      messages: [
        { role: "user", content: "Continue with restrained dread." },
        { role: "assistant", content: "The candle guttered." },
      ],
    },
  },
};

test("parsePersistedEditorState falls back for missing or malformed storage", () => {
  const fallback = initialPersistedEditorState();

  expect(parsePersistedEditorState(null)).toEqual(fallback);
  expect(parsePersistedEditorState("not json")).toEqual(fallback);
  expect(parsePersistedEditorState(JSON.stringify([]))).toEqual(fallback);
  expect(parsePersistedEditorState('{"aiTextTint":"false"}').aiTextTint).toBe(true);
  expect(parsePersistedEditorState('{"showContinueWindow":"false"}').showContinueWindow).toBe(true);
});

test("persisted editor state round trips document, controls, and thread history", () => {
  const serialized = serializePersistedEditorState(persistedState);

  expect(parsePersistedEditorState(serialized)).toEqual(persistedState);
  expect(JSON.parse(serialized)).not.toHaveProperty("instruction");
});

test("legacy instruction migrates only to Continue", () => {
  const parsed = parsePersistedEditorState(JSON.stringify({ instruction: "Legacy guidance" }));
  expect(parsed.continueInstruction).toBe("Legacy guidance");
  expect(parsed.editInstruction).toBe("");
});

test("separate instructions take precedence over legacy instruction", () => {
  const parsed = parsePersistedEditorState(JSON.stringify({ ...persistedState, instruction: "Legacy guidance" }));
  expect(parsed).toEqual(persistedState);
  expect(parsePersistedEditorState('{"continueInstruction":"","instruction":"Legacy guidance"}').continueInstruction).toBe("");
});

test("invalid instructions fall back to defaults", () => {
  for (const value of [null, 12, [], {}]) {
    const parsed = parsePersistedEditorState(JSON.stringify({
      editInstruction: value, continueInstruction: value, instruction: "Legacy guidance",
    }));
    expect(parsed.editInstruction).toBe("");
    expect(parsed.continueInstruction).toBe("");
    expect(parsePersistedEditorState(JSON.stringify({ instruction: value })).continueInstruction).toBe("");
  }
});

test("invalid nested thread state falls back safely", () => {
  const parsed = parsePersistedEditorState(
    JSON.stringify({
      ...persistedState,
      threadState: {
        settings: { maxOutputTokens: 900, resumePrompt: DEFAULT_RESUME_PROMPT },
        thread: {
          kind: "resumable_generate",
          instruction: "Continue",
          generatedRange: { start: 0, end: 10 },
          messages: [{ role: "system", content: "invalid" }],
        },
      },
    }),
  );

  expect(parsed).toEqual({
    ...persistedState,
    threadState: initialPersistedEditorState().threadState,
  });
});

test("legacy global thread is dropped while settings and scoped threads survive", () => {
  const parsed = parsePersistedEditorState(JSON.stringify({
    ...persistedState,
    threadState: { ...persistedState.threadState, thread: persistedState.threads["chapter:1:2:3"] },
  }));
  expect(parsed.threadState.thread).toBeNull();
  expect(parsed.threadState.settings).toEqual(persistedState.threadState.settings);
  expect(parsed.threads).toEqual(persistedState.threads);
});
