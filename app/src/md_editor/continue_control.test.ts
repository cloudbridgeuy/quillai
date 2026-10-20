import { expect, test } from "bun:test";
import { continueClickAction } from "./editor_ai";
import { finishGenerateThread, initialAiThreadState, shouldContinueWithThread } from "./ai_thread";
import type { AiRunState } from "./ai_state";

test("Continue indicates only the history that will be sent", () => {
  const empty = initialAiThreadState(512);
  const saved = finishGenerateThread({
    state: empty, finishReason: "length", instruction: "Write", userPrompt: "Write",
    generatedText: "A beginning", generatedRange: { start: 0, end: 11 },
  });
  expect(shouldContinueWithThread(empty, "Write")).toBe(false);
  expect(shouldContinueWithThread(saved, "Write")).toBe(true);
  expect(shouldContinueWithThread(saved, "Changed")).toBe(false);
  expect(shouldContinueWithThread(saved, "Write", [])).toBe(false);
  expect(shouldContinueWithThread(empty, null, [{ role: "user", text: "Earlier" }])).toBe(true);
  if (!saved.thread) throw new Error("Expected saved thread");
  expect(shouldContinueWithThread({ ...saved, thread: { ...saved.thread, messages: [] } }, "Write")).toBe(false);
});

test("Continue clicks stop running streams and start only outside a run", () => {
  const running: AiRunState[] = [
    { status: "continuing", action: "generate" },
    { status: "editing", action: "edit" },
    { status: "cancelling", action: "generate" },
    { status: "cancelling", action: "edit" },
  ];
  for (const state of running) expect(continueClickAction(state)).toBe("stop");
  const ready: AiRunState[] = [
    { status: "idle" },
    { status: "finished", action: "generate", outcome: "done" },
    { status: "finished", action: "edit", outcome: "cancelled" },
    { status: "failed", action: "generate", error: "Failed" },
  ];
  for (const state of ready) expect(continueClickAction(state)).toBe("generate");
});
