import { useEffect, useRef, type RefObject } from "react";

export type OutlineFocusTarget = "previous" | "row" | "region" | "none";

export function outlineFocusTarget(state: { focusWasInOutline: boolean; activeElementIsBody: boolean; hasPreviousElement: boolean; previousWasTextEntry?: boolean; hasPressedRow: boolean }): OutlineFocusTarget {
  if (!state.focusWasInOutline || !state.activeElementIsBody) return "none";
  if (state.hasPreviousElement) return "previous";
  return state.hasPressedRow && !state.previousWasTextEntry ? "row" : "region";
}

export function useRestoreOutlineFocus(outlineRef: RefObject<HTMLElement | null>, pending: boolean): void {
  const lastInside = useRef<HTMLElement | null>(null);
  const wasInside = useRef(false);
  const wasPending = useRef(false);
  useEffect(() => {
    const track = (event: FocusEvent) => {
      if (wasPending.current) return;
      const inside = event.target instanceof HTMLElement && Boolean(outlineRef.current?.contains(event.target));
      wasInside.current = inside;
      lastInside.current = inside ? (event.target as HTMLElement) : null;
    };
    document.addEventListener("focusin", track);
    return () => document.removeEventListener("focusin", track);
  }, [outlineRef]);
  useEffect(() => {
    const was = wasPending.current;
    wasPending.current = pending;
    if (pending || !was) return;
    const outline = outlineRef.current;
    const previous = lastInside.current;
    const usable = previous !== null && previous.isConnected && Boolean(outline?.contains(previous));
    const row = outline?.querySelector<HTMLElement>('button[aria-pressed="true"]') ?? null;
    const target = outlineFocusTarget({ focusWasInOutline: wasInside.current, activeElementIsBody: !document.activeElement || document.activeElement === document.body, hasPreviousElement: usable, previousWasTextEntry: previous?.matches("input, textarea"), hasPressedRow: row !== null });
    wasInside.current = false;
    lastInside.current = null;
    if (target === "previous") previous?.focus();
    else if (target === "row") row?.focus();
    else if (target === "region") outline?.focus();
  }, [outlineRef, pending]);
}
