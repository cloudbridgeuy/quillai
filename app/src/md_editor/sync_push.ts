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
  let stream: ReturnType<typeof createSyncStream> | null = null;
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
    if (closed || state.suspended) return;
    const path = "pieceId" in scope ? undefined : `/series/${scope.seriesId}/books/${scope.bookId}/chapters/${scope.chapterId}/events`;
    stream = createSyncStream(pieceId === null ? [] : [pieceId], {
      ready(ids) { pieceId = ids[0]!; state = requestRefetch(state); check(); },
      refetch() { state = requestRefetch(state); check(); },
      changed(id, revision) {
        if (id !== pieceId || revision <= session.revision() || revision <= state.revision) return;
        state = requestRevision(state, revision);
        check();
      },
    }, path);
  };
  const disconnect = () => { stream?.close(); stream = null; };
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

export function createSyncStream(ids: readonly number[], handlers: {
  ready?: (ids: number[]) => void;
  refetch: () => void;
  changed: (id: number, revision: number) => void;
}, path = `/sync/events?pieceIds=${ids.join(",")}`) {
  let source: EventSource | null = null;
  let activeIds = ids;
  if (typeof EventSource !== "undefined") {
    try {
      const stream = new EventSource(`${getApiBaseUrl()}${path}`);
      source = stream;
      stream.addEventListener("ready", (event) => {
        if (source !== stream) return;
        try {
          const received = JSON.parse((event as MessageEvent).data).pieceIds;
          if (!Array.isArray(received) || received.length !== (activeIds.length || 1) || !received.every((id: unknown) => Number.isSafeInteger(id) && (id as number) > 0) || (activeIds.length > 0 && (new Set(received).size !== activeIds.length || activeIds.some((id) => !received.includes(id))))) return;
          activeIds = received;
          if (handlers.ready) handlers.ready(received);
          else handlers.refetch();
        } catch {}
      });
      stream.addEventListener("resync", () => { if (source === stream) handlers.refetch(); });
      stream.addEventListener("changed", (event) => {
        if (source !== stream) return;
        try {
          const data = JSON.parse((event as MessageEvent).data);
          if (activeIds.includes(data.pieceId) && Number.isSafeInteger(data.rev) && data.rev > 0) handlers.changed(data.pieceId, data.rev);
        } catch {}
      });
    } catch {}
  }
  return { close() { source?.close(); source = null; } };
}
