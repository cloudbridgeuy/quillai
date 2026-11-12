import type { Meta, StoryObj } from "@storybook/react-vite";
import { IconButton } from "./IconButton";

const meta = {
  title: "UI/IconButton",
  component: IconButton,
  args: { label: "Bookmark", glyph: "★" },
} satisfies Meta<typeof IconButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: ({ canvasElement }) => {
    const name = canvasElement.querySelector("button")!.getAttribute("aria-label");
    if (name !== "Bookmark") throw new Error(`expected accessible name Bookmark, got ${name}`);
  },
};

export const Pressed: Story = {
  args: { pressed: true },
  play: ({ canvasElement }) => {
    if (canvasElement.querySelector("button")!.getAttribute("aria-pressed") !== "true") throw new Error("expected aria-pressed=true");
  },
};

export const WithBadge: Story = {
  name: "With badge",
  args: { badge: <span className="text-micro">3</span> },
  play: ({ canvasElement }) => {
    const button = canvasElement.querySelector("button");
    if (button?.getAttribute("aria-label") !== "Bookmark") throw new Error("badge must not replace the accessible name");
    if (button.querySelector("span")?.textContent !== "3") throw new Error("badge missing");
  },
};

export const Continue: Story = {
  args: { kind: "continue", label: "Continue", glyph: "→" },
  play: ({ canvasElement }) => {
    const button = canvasElement.querySelector("button");
    if (!button) throw new Error("missing button");
    if (button.getAttribute("aria-label") !== "Continue") throw new Error(`expected accessible name Continue, got ${button.getAttribute("aria-label")}`);
    if (button.getAttribute("title") !== "Continue") throw new Error(`expected tooltip Continue, got ${button.getAttribute("title")}`);
    if (button.textContent?.includes("Continue")) throw new Error(`expected icon only, got ${button.textContent}`);
    if (!button.className.includes("size-9")) throw new Error("expected the shared icon size");
  },
};

export const ContinueBadge: Story = {
  name: "Continue badge",
  args: { kind: "continue", label: "Continue", glyph: "→", badge: <span role="img" aria-label="from a cut-off reply" /> },
  play: ({ canvasElement }) => {
    const button = canvasElement.querySelector("button");
    if (button?.getAttribute("aria-label") !== "Continue") throw new Error("badge must not replace the accessible name");
    if (button.querySelector("[role=img]")?.getAttribute("aria-label") !== "from a cut-off reply") throw new Error("badge missing");
    if (button.textContent?.includes("Continue")) throw new Error("expected icon only");
  },
};

export const Disabled: Story = { args: { disabled: true } };
