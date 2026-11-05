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

export const WithBadge: Story = { name: "With badge", args: { badge: <span className="text-micro">3</span> } };

export const Continue: Story = { args: { kind: "continue", label: "Continue", glyph: "→" } };

export const Disabled: Story = { args: { disabled: true } };
