import { describe, expect, test } from "bun:test";
import { checkPush, createSyncPush, pushState, requestRefetch, requestRevision } from "./sync_push";

const idle = { revision: 4, pending: false, busy: false, reserved: false };

describe("push scheduler", () => {
  test("skips clean idle checks and polls at 15 seconds", () => {
    const start = pushState(0);
    expect(checkPush(start, idle, 14_999).tick).toBe(false);
    const due = checkPush(start, idle, 15_000);
    expect(due.tick).toBe(true);
    expect(checkPush(due.state, idle, 29_999).tick).toBe(false);
    expect(checkPush(due.state, idle, 30_000).tick).toBe(true);
  });

  test("runs pending saves and retries on checks", () => {
    expect(checkPush(pushState(0), { ...idle, pending: true }, 1000).tick).toBe(true);
  });

  test("ready and resync request immediate reads and reset fallback", () => {
    const ready = checkPush(requestRefetch(pushState(0)), idle, 100);
    expect(ready.tick).toBe(true);
    expect(checkPush(ready.state, idle, 15_000).tick).toBe(false);
    expect(checkPush(requestRefetch(ready.state), idle, 200).tick).toBe(true);
  });

  test("drops covered nudges and coalesces ahead revisions while unavailable", () => {
    expect(checkPush(requestRevision(pushState(0), 4), idle, 10).tick).toBe(false);
    let result = checkPush(requestRevision(requestRevision(pushState(0), 6), 5), { ...idle, busy: true }, 10);
    expect(result.tick).toBe(false);
    expect(result.state.revision).toBe(6);
    result = checkPush(result.state, { ...idle, reserved: true }, 20);
    expect(result.tick).toBe(false);
    expect(checkPush(result.state, { ...idle, revision: 6 }, 30).tick).toBe(false);
    expect(checkPush(result.state, idle, 30).tick).toBe(true);
  });

  test("retains refetch through busy and suspension", () => {
    const state = requestRefetch(pushState(0));
    expect(checkPush(state, { ...idle, busy: true }, 1).state.refetch).toBe(true);
    expect(checkPush({ ...state, suspended: true }, idle, 20_000).tick).toBe(false);
    expect(checkPush(state, idle, 20_000).tick).toBe(true);
  });

  test("retains the advertised head until a read confirms it", () => {
    const first = checkPush(requestRevision(pushState(0), 6), idle, 10);
    expect(first.tick).toBe(true);
    expect(first.state.revision).toBe(6);
    const busy = checkPush(first.state, { ...idle, busy: true }, 20);
    expect(busy.tick).toBe(false);
    const missed = checkPush(busy.state, { ...idle, revision: 5 }, 1000);
    expect(missed.tick).toBe(true);
    expect(missed.state.revision).toBe(6);
    const confirmed = checkPush(missed.state, { ...idle, revision: 6 }, 2000);
    expect(confirmed.tick).toBe(false);
    expect(confirmed.state.revision).toBe(0);
  });

  test("no event leaves the fallback deadline unchanged", () => {
    const state = pushState(0);
    expect(checkPush(state, idle, 1000).state.deadline).toBe(15_000);
    expect(checkPush(state, idle, 15_000).tick).toBe(true);
  });
});

test("stream events and lifecycle request reads without timers", () => {
  const original = globalThis.EventSource;
  class FakeSource extends EventTarget {
    static instances: FakeSource[] = [];
    closed = false;
    constructor(readonly url: string) { super(); FakeSource.instances.push(this); }
    close() { this.closed = true; }
    emit(name: string, data: unknown) { this.dispatchEvent(new MessageEvent(name, { data: JSON.stringify(data) })); }
  }
  globalThis.EventSource = FakeSource as unknown as typeof EventSource;
  try {
    let revision = 4;
    let busy = false;
    let pending = false;
    let ticks = 0;
    const push = createSyncPush({ seriesId: 1, bookId: 2, chapterId: 3 }, {
      revision: () => revision, hasPendingWork: () => pending, isBusy: () => busy, isReserved: () => false,
      tick: async () => { ticks++; },
    }, () => false);
    const first = FakeSource.instances[0]!;
    expect(first.url.endsWith("/series/1/books/2/chapters/3/events")).toBe(true);
    first.emit("ready", { pieceIds: [9] });
    expect(ticks).toBe(1);
    first.emit("changed", { pieceId: 9, rev: 4 });
    expect(ticks).toBe(1);
    busy = true;
    first.emit("changed", { pieceId: 9, rev: 6 });
    first.emit("changed", { pieceId: 9, rev: 5 });
    busy = false;
    revision = 6;
    push.check();
    expect(ticks).toBe(1);
    first.emit("resync", {});
    expect(ticks).toBe(2);
    first.dispatchEvent(new Event("error"));
    expect(ticks).toBe(2);
    push.hide();
    expect(first.closed).toBe(true);
    push.check();
    expect(ticks).toBe(2);
    push.show();
    expect(FakeSource.instances).toHaveLength(2);
    expect(ticks).toBe(3);
    push.close();
    expect(FakeSource.instances[1]!.closed).toBe(true);
    pending = true;
    push.check();
    expect(ticks).toBe(3);
    const hidden = createSyncPush({ universeId: 1, pieceId: 9 }, {
      revision: () => revision, hasPendingWork: () => false, isBusy: () => false, isReserved: () => false,
      tick: async () => { ticks++; },
    }, () => false, true);
    hidden.check();
    expect(ticks).toBe(3);
    expect(FakeSource.instances).toHaveLength(2);
    hidden.show();
    expect(FakeSource.instances).toHaveLength(3);
    expect(FakeSource.instances[2]!.url.endsWith("/sync/events?pieceIds=9")).toBe(true);
    expect(ticks).toBe(4);
    hidden.close();
  } finally {
    globalThis.EventSource = original;
  }
});
