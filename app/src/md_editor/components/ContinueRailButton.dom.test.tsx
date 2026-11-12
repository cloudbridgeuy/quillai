import { afterEach, expect, test } from "bun:test";
import { cleanup, render } from "../../test/dom";
import { ContinueRailButton } from "./ContinueRailButton";

afterEach(cleanup);

const noop = () => {};

test("Continue is an icon named Continue, including while saving", async () => {
  const idle = await render(<ContinueRailButton saving={false} busy={false} cutOff={false} modelName={null} disabled={false} onMouseDown={noop} onClick={noop} />);
  const button = idle.querySelector("button")!;
  expect(button.getAttribute("aria-label")).toBe("Continue");
  expect(button.getAttribute("title")).toBe("Continue (Ctrl/Cmd+Enter)");
  expect(button.textContent).toBe("➤");
  expect(idle.querySelectorAll("button")).toHaveLength(1);

  const saving = await render(<ContinueRailButton saving busy={false} cutOff={false} modelName="Precise" disabled={false} onMouseDown={noop} onClick={noop} />);
  const savingButton = saving.querySelector("button")!;
  expect(savingButton.getAttribute("aria-label")).toBe("Continue");
  expect(savingButton.getAttribute("title")).toBe("Saving… (Ctrl/Cmd+Enter) · Precise");
  expect(savingButton.textContent).toBe("➤");
});

test("Stop replaces the Continue icon in the same rail slot", async () => {
  const container = await render(<ContinueRailButton saving={false} busy cutOff={false} modelName={null} disabled={false} onMouseDown={noop} onClick={noop} />);
  const button = container.querySelector("button")!;
  expect(container.querySelectorAll("button")).toHaveLength(1);
  expect(button.className).toContain("editor-continue");
  expect(button.getAttribute("aria-label")).toBe("Stop AI");
  expect(button.getAttribute("title")).toBe("Stop AI (Ctrl/Cmd+Enter)");
  expect(button.textContent).toBe("■");

  const saving = await render(<ContinueRailButton saving busy cutOff modelName="Precise" disabled={false} onMouseDown={noop} onClick={noop} />);
  const savingButton = saving.querySelector("button")!;
  expect(saving.querySelectorAll("button")).toHaveLength(1);
  expect(savingButton.getAttribute("aria-label")).toBe("Stop AI");
  expect(savingButton.getAttribute("title")).toBe("Saving… (Ctrl/Cmd+Enter) · Precise");
  expect(savingButton.textContent).toBe("■");
  expect(savingButton.querySelector(".editor-thread-indicator")?.getAttribute("aria-label")).toBe("from a cut-off reply");
});

test("a cut-off reply is a badge with a tooltip while Continue keeps its name", async () => {
  const container = await render(<ContinueRailButton saving={false} busy={false} cutOff modelName={null} disabled={false} onMouseDown={noop} onClick={noop} />);
  const button = container.querySelector("button")!;
  expect(button.getAttribute("aria-label")).toBe("Continue");
  expect(button.getAttribute("title")).toBe("Continue (Ctrl/Cmd+Enter)");
  const badge = button.querySelector(".editor-thread-indicator");
  expect(badge?.getAttribute("aria-label")).toBe("from a cut-off reply");
  expect(badge?.getAttribute("title")).toBe("Continue writing from a cut-off reply");
  expect(badge?.textContent).toBe("");
});
