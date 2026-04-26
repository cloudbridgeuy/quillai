import type { AiAction } from "./ai_contract";

export type AiRunState =
  | { status: "idle" }
  | { status: "continuing"; action: "generate" }
  | { status: "editing"; action: "edit" }
  | { status: "cancelling"; action: AiAction }
  | { status: "failed"; action: AiAction | null; error: string }
  | { status: "finished"; action: AiAction; outcome: "done" | "cancelled" };

export type AiStateTransitionResult =
  | { ok: true; state: AiRunState }
  | { ok: false; reason: string };

export const idleAiRunState: AiRunState = { status: "idle" };

export function startContinuing(state: AiRunState): AiStateTransitionResult {
  if (!isIdle(state)) {
    return invalidTransition(state, "continuing");
  }

  return success({ status: "continuing", action: "generate" });
}

export function startEditing(state: AiRunState): AiStateTransitionResult {
  if (!isIdle(state)) {
    return invalidTransition(state, "editing");
  }

  return success({ status: "editing", action: "edit" });
}

export function startCancelling(state: AiRunState): AiStateTransitionResult {
  if (!isRunning(state)) {
    return invalidTransition(state, "cancelling");
  }

  return success({ status: "cancelling", action: state.action });
}

export function finishRun(
  state: AiRunState,
  outcome: "done" | "cancelled",
): AiStateTransitionResult {
  if (isRunning(state) || state.status === "cancelling") {
    return success({ status: "finished", action: state.action, outcome });
  }

  return invalidTransition(state, `finished:${outcome}`);
}

export function failRun(
  state: AiRunState,
  error: string,
): AiStateTransitionResult {
  if (!canFail(state)) {
    return invalidTransition(state, "failed");
  }

  return success({
    status: "failed",
    action: state.action,
    error: normalizeErrorMessage(error),
  });
}

export function resetRunState(state: AiRunState): AiStateTransitionResult {
  if (!isTerminal(state)) {
    return invalidTransition(state, "idle");
  }

  return success(idleAiRunState);
}

function normalizeErrorMessage(error: string): string {
  const trimmed = error.trim();
  return trimmed === "" ? "Unknown AI run failure" : trimmed;
}

function success(state: AiRunState): AiStateTransitionResult {
  return { ok: true, state };
}

function isIdle(state: AiRunState): state is Extract<AiRunState, { status: "idle" }> {
  return state.status === "idle";
}

function isRunning(
  state: AiRunState,
): state is Extract<AiRunState, { status: "continuing" | "editing" }> {
  return state.status === "continuing" || state.status === "editing";
}

function canFail(
  state: AiRunState,
): state is Extract<AiRunState, { status: "continuing" | "editing" | "cancelling" }> {
  return isRunning(state) || state.status === "cancelling";
}

function isTerminal(
  state: AiRunState,
): state is Extract<AiRunState, { status: "finished" | "failed" }> {
  return state.status === "finished" || state.status === "failed";
}

function invalidTransition(state: AiRunState, nextStatus: string): AiStateTransitionResult {
  return {
    ok: false,
    reason: `cannot transition from ${state.status} to ${nextStatus}`,
  };
}
