import { getApiBaseUrl } from "../api_config";
import type { EditorScope } from "../manuscripts/contract";

type Facts = { revision: number; pending: boolean; busy: boolean; reserved: boolean };
export type PushState = { deadline: number; revision: number; refetch: boolean; suspended: boolean };

export function pushState(now: number): PushState {
  return { deadline: now + 15_000, revision: 0, refetch: false, suspended: false };
}

export function requestRefetch(state: PushState): PushState {
  return { ...state, refetch: true };
}

export function requestRevision(state: PushState, revision: number): PushState {
  return { ...state, revision: Math.max(state.revision, revision) };
}

export function checkPush(state: PushState, facts: Facts, now: number): { state: PushState; tick: boolean } {
  const revision = state.revision > facts.revision ? state.revision : 0;
  if (state.suspended || facts.busy || facts.reserved) return { state: { ...state, revision }, tick: false };
  if (!facts.pending && !state.refetch && !revision && now < state.deadline) return { state: { ...state, revision }, tick: false };
  return { state: { ...state, deadline: now + 15_000, revision, refetch: false }, tick: true };
}

type Session = { revision(): number; hasPendingWork(): boolean; isBusy(): boolean; isReserved(): boolean; tick(): Promise<void> };

export function createSyncPush(scope: EditorScope, session: Session, unavailable: () => boolean, suspended = false) {
  let state = { ...pushState(Date.now()), suspended };
  let source: EventSource | null = null;
  let pieceId = "pieceId" in scope ? scope.pieceId : null;
  let closed = false;
  const check = () => {
    if (closed) return;
    const result = checkPush(state, {
      revision: session.revision(), pending: session.hasPendingWork(),
      busy: session.isBusy(), reserved: session.isReserved() || unavailable(),
    }, Date.now());
    state = result.state;
    if (result.tick) void session.tick();
  };
  const open = () => {
    if (closed || state.suspended || typeof EventSource === "undefined") return;
    const path = "pieceId" in scope
      ? `/sync/events?pieceIds=${scope.pieceId}`
      : `/series/${scope.seriesId}/books/${scope.bookId}/chapters/${scope.chapterId}/events`;
    try {
      const stream = new EventSource(`${getApiBaseUrl()}${path}`);
      source = stream;
      stream.addEventListener("ready", (event) => {
        if (source !== stream) return;
        try {
          const ids = JSON.parse((event as MessageEvent).data).pieceIds;
          if (!Array.isArray(ids) || ids.length !== 1 || !Number.isSafeInteger(ids[0]) || ids[0] <= 0 || (pieceId !== null && ids[0] !== pieceId)) return;
          pieceId = ids[0];
          state = requestRefetch(state);
          check();
        } catch {}
      });
      stream.addEventListener("resync", () => {
        if (source !== stream) return;
        state = requestRefetch(state);
        check();
      });
      stream.addEventListener("changed", (event) => {
        if (source !== stream) return;
        try {
          const data = JSON.parse((event as MessageEvent).data);
          if (pieceId === null || data.pieceId !== pieceId || !Number.isSafeInteger(data.rev) || data.rev <= session.revision() || data.rev <= state.revision) return;
          state = requestRevision(state, data.rev);
          check();
        } catch {}
      });
    } catch {}
  };
  const disconnect = () => { source?.close(); source = null; };
  open();
  return {
    check,
    hide() { state = { ...state, suspended: true }; disconnect(); },
    show() {
      if (closed || !state.suspended) return;
      state = requestRefetch({ ...state, suspended: false });
      open();
      check();
    },
    close() { closed = true; disconnect(); },
  };
}
