import { afterEach, expect, test } from "bun:test";
import { cleanup, click, render } from "../test/dom";
import { Tips } from "./LandingTips";
import { TIPS, TIPS_DISMISSED_KEY } from "./tips";

afterEach(async () => { await cleanup(); window.localStorage.clear(); });

test("shows tips until dismissed and keeps them hidden on remount", async () => {
  const container = await render(<Tips />);
  expect(container.textContent).toContain("Tips");
  for (const tip of TIPS) expect(container.textContent).toContain(tip);
  await click(container.querySelector("button")!);
  expect(container.textContent).toBe("");
  expect(window.localStorage.getItem(TIPS_DISMISSED_KEY)).toBe("true");
  const remounted = await render(<Tips />);
  expect(remounted.textContent).toBe("");
});
