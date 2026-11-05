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
