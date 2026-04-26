import { expect, test } from "bun:test";

import {
  failRun,
  finishRun,
  idleAiRunState,
  resetRunState,
  startCancelling,
  startContinuing,
  startEditing,
} from "./ai_state";

test("startContinuing transitions idle to continuing", () => {
  expect(startContinuing(idleAiRunState)).toEqual({
    ok: true,
    state: { status: "continuing", action: "generate" },
  });
});

test("startEditing transitions idle to editing", () => {
  expect(startEditing(idleAiRunState)).toEqual({
    ok: true,
    state: { status: "editing", action: "edit" },
  });
});

test("startCancelling transitions a running state to cancelling", () => {
  expect(startCancelling({ status: "continuing", action: "generate" })).toEqual({
    ok: true,
    state: { status: "cancelling", action: "generate" },
  });
});

test("failRun transitions a running state to failed", () => {
  expect(failRun({ status: "editing", action: "edit" }, " boom ")).toEqual({
    ok: true,
    state: { status: "failed", action: "edit", error: "boom" },
  });
});

test("finishRun transitions a running state to finished", () => {
  expect(finishRun({ status: "continuing", action: "generate" }, "done")).toEqual({
    ok: true,
    state: { status: "finished", action: "generate", outcome: "done" },
  });
});

test("resetRunState transitions a terminal state back to idle", () => {
  expect(
    resetRunState({ status: "finished", action: "edit", outcome: "cancelled" }),
  ).toEqual({
    ok: true,
    state: idleAiRunState,
  });
});

test("finishRun transitions cancelling to finished when cancellation completes", () => {
  expect(finishRun({ status: "cancelling", action: "edit" }, "cancelled")).toEqual({
    ok: true,
    state: { status: "finished", action: "edit", outcome: "cancelled" },
  });
});

test("finishRun allows a done outcome to arrive after cancellation starts", () => {
  expect(finishRun({ status: "cancelling", action: "edit" }, "done")).toEqual({
    ok: true,
    state: { status: "finished", action: "edit", outcome: "done" },
  });
});

test("failRun transitions cancelling to failed", () => {
  expect(failRun({ status: "cancelling", action: "generate" }, "   ")).toEqual({
    ok: true,
    state: {
      status: "failed",
      action: "generate",
      error: "Unknown AI run failure",
    },
  });
});

test("resetRunState transitions failed back to idle", () => {
  expect(
    resetRunState({ status: "failed", action: null, error: "boom" }),
  ).toEqual({
    ok: true,
    state: idleAiRunState,
  });
});

test("invalid transitions are rejected across uncovered paths", () => {
  expect(startContinuing({ status: "failed", action: "edit", error: "boom" })).toEqual({
    ok: false,
    reason: "cannot transition from failed to continuing",
  });

  expect(startEditing({ status: "continuing", action: "generate" })).toEqual({
    ok: false,
    reason: "cannot transition from continuing to editing",
  });

  expect(startCancelling(idleAiRunState)).toEqual({
    ok: false,
    reason: "cannot transition from idle to cancelling",
  });

  expect(finishRun(idleAiRunState, "cancelled")).toEqual({
    ok: false,
    reason: "cannot transition from idle to finished:cancelled",
  });

  expect(failRun(idleAiRunState, "boom")).toEqual({
    ok: false,
    reason: "cannot transition from idle to failed",
  });

  expect(resetRunState({ status: "cancelling", action: "edit" })).toEqual({
    ok: false,
    reason: "cannot transition from cancelling to idle",
  });
});
