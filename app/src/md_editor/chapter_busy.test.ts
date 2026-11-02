import { describe, expect, test } from "bun:test";
import { scheduleChapterBusy, showChapterBusy } from "./chapter_busy";

describe("chapter busy delay", () => {
  test("appears at 300 ms", () => {
    expect(showChapterBusy(1000, 1299)).toBe(false);
    expect(showChapterBusy(1000, 1300)).toBe(true);
  });

  test("timer shows busy at the threshold and can be cancelled", () => {
    let now = 0;
    let due = 0;
    let callback: (() => void) | undefined;
    let shown = 0;
    const schedule = ((fn: () => void, delay: number) => { callback = fn; due = now + delay; return 1; }) as unknown as typeof setTimeout;
    const cancel = (() => { callback = undefined; }) as typeof clearTimeout;
    const advance = (time: number) => { now = time; if (callback && now >= due) { const run = callback; callback = undefined; run(); } };
    const finish = scheduleChapterBusy(0, () => shown++, schedule, cancel, () => now);
    advance(299);
    expect(shown).toBe(0);
    advance(300);
    expect(shown).toBe(1);
    finish();
    const earlyFinish = scheduleChapterBusy(300, () => shown++, schedule, cancel, () => now);
    earlyFinish();
    advance(600);
    expect(shown).toBe(1);
  });
});
