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
  instruction: "Continue with restrained dread.",
  windowSize: "2400",
  windowMode: "document",
  threadState: {
    settings: {
      maxOutputTokens: 900,
      resumePrompt: "Keep going from the last sentence.",
    },
    thread: {
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
});

test("persisted editor state round trips document, controls, and thread history", () => {
  const serialized = serializePersistedEditorState(persistedState);

  expect(parsePersistedEditorState(serialized)).toEqual(persistedState);
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
