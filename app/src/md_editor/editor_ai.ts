import type { AiAction, AiSelection, DeltaDocument } from "./ai_contract";
import type { AiRunState } from "./ai_state";

export type StatusPresentation = {
  tone: "idle" | "running" | "success" | "warning" | "error";
  label: string;
};

export function rememberEditSelection(
  previous: AiSelection | null,
  next: AiSelection | null,
): AiSelection | null {
  if (next === null) {
    return previous;
  }

  if (next.length === 0) {
    return null;
  }

  return next;
}

export function rememberCursorRange(
  previous: AiSelection | null,
  next: AiSelection | null,
): AiSelection | null {
  return next ?? previous;
}

export function resolveContinueSelection(
  current: AiSelection | null,
  remembered: AiSelection | null,
): AiSelection | null {
  const active = current ?? remembered;
  if (!active) {
    return null;
  }

  return {
    index: active.index + active.length,
    length: 0,
  };
}

export function resolveEditSelection(
  current: AiSelection | null,
  remembered: AiSelection | null,
): AiSelection | null {
  return current && current.length > 0 ? current : remembered;
}

export function shouldRestoreCancelledEditSelection(
  action: AiAction,
  didDeleteSelection: boolean,
  deletedLength: number,
  insertedLength: number,
): boolean {
  return (
    action === "edit" && didDeleteSelection && deletedLength > 0 && insertedLength === 0
  );
}

export function shouldRollbackFailedEditSelection(
  action: AiAction,
  didDeleteSelection: boolean,
  deletedLength: number,
): boolean {
  return action === "edit" && didDeleteSelection && deletedLength > 0;
}

export function shouldPreservePendingEditSelection(
  action: AiAction,
  didDeleteSelection: boolean,
  deletedLength: number,
): boolean {
  return action === "edit" && !didDeleteSelection && deletedLength > 0;
}

export function shouldClearFailedRunState(
  state: AiRunState,
  source: string,
): boolean {
  return state.status === "failed" && source === "user";
}

export function isTextOnlyDeltaDocument(delta: DeltaDocument): boolean {
  return delta.ops.every((op) => typeof op.insert === "string");
}

export function getStatusPresentation(state: AiRunState): StatusPresentation {
  switch (state.status) {
    case "continuing":
      return { tone: "running", label: "Continuing" };
    case "editing":
      return { tone: "running", label: "Editing" };
    case "cancelling":
      return { tone: "running", label: "Cancelling" };
    case "finished":
      if (state.outcome === "cancelled") {
        return { tone: "warning", label: "Cancelled" };
      }

      return { tone: "success", label: "Done" };
    case "failed":
      return { tone: "error", label: state.error };
    case "idle":
      return { tone: "idle", label: "Idle" };
  }
}
