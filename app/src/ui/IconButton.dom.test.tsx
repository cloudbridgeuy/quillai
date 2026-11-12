import { afterEach, expect, test } from "bun:test";
import { cleanup, render } from "../test/dom";
import { IconButton } from "./IconButton";

afterEach(cleanup);

test("Continue is an icon with its name in the tooltip and aria-label", async () => {
  const container = await render(<IconButton kind="continue" label="Continue" glyph="➤" />);
  const button = container.querySelector("button");
  expect(button).not.toBeNull();
  expect(button!.getAttribute("aria-label")).toBe("Continue");
  expect(button!.getAttribute("title")).toBe("Continue");
  expect(button!.textContent).toBe("➤");
  expect(button!.className).toContain("size-9");
  expect(button!.className).not.toContain("w-14");
});

test("a Continue badge keeps the icon name and accepts a tooltip override", async () => {
  const container = await render(<IconButton kind="continue" label="Continue" glyph="➤" title="Saving…" badge={<span role="img" aria-label="from a cut-off reply" />} />);
  const button = container.querySelector("button")!;
  expect(button.getAttribute("aria-label")).toBe("Continue");
  expect(button.getAttribute("title")).toBe("Saving…");
  expect(button.textContent).toBe("➤");
  expect(button.querySelector("[role=img]")?.getAttribute("aria-label")).toBe("from a cut-off reply");
});
