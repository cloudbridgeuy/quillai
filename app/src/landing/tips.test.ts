import { expect, test } from "bun:test";
import { TIPS, TIPS_DISMISSED_KEY, readTipsDismissed, dismissTips } from "./tips";

test("provides four or five tips and the versioned storage key", () => {
  expect(TIPS.length).toBeGreaterThanOrEqual(4);
  expect(TIPS.length).toBeLessThanOrEqual(5);
  expect(TIPS.every(tip => tip.length > 0)).toBe(true);
  expect(TIPS_DISMISSED_KEY).toBe("quillai.landing.tipsDismissed.v1");
});

test("reads and writes dismissal and tolerates storage errors", () => {
  const data = new Map<string, string>();
  const storage = { getItem: (key: string) => data.get(key) ?? null, setItem: (key: string, value: string) => { data.set(key, value); } };
  expect(readTipsDismissed(storage)).toBe(false);
  dismissTips(storage);
  expect(data.get(TIPS_DISMISSED_KEY)).toBe("true");
  expect(readTipsDismissed(storage)).toBe(true);
  const broken = { getItem: () => { throw new Error(); }, setItem: () => { throw new Error(); } };
  expect(readTipsDismissed(broken)).toBe(false);
  expect(() => dismissTips(broken)).not.toThrow();
});
