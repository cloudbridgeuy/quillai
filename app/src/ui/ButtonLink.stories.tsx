import type { Meta, StoryObj } from "@storybook/react-vite";
import { ButtonLink } from "./ButtonLink";

const meta = {
  title: "UI/ButtonLink",
  component: ButtonLink,
  args: { href: "#", children: "Open manuscript", variant: "quiet", size: "md" },
} satisfies Meta<typeof ButtonLink>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Quiet: Story = {};

export const Primary: Story = { args: { variant: "primary" } };

export const Danger: Story = { args: { variant: "danger", children: "Delete series" } };

export const Link: Story = { args: { variant: "link" } };

export const Small: Story = { args: { size: "sm" } };

export const Anchor: Story = {
  args: { href: "/manuscripts/1" },
  play: ({ canvasElement }) => {
    const link = canvasElement.querySelector("a");
    if (link == null) throw new Error("link did not render");
    if (link.getAttribute("href") !== "/manuscripts/1") throw new Error("link href missing");
    if (link.textContent?.trim() !== "Open manuscript") throw new Error("link accessible name missing");
  },
};
