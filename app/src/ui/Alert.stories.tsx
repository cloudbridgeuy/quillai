import type { Meta, StoryObj } from "@storybook/react-vite";
import { Alert } from "./Alert";

const meta = {
  title: "UI/Alert",
  component: Alert,
  args: { tone: "error", children: "The chapter could not be saved." },
} satisfies Meta<typeof Alert>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ErrorPill: Story = { name: "Error" };

export const Warning: Story = { args: { tone: "warning", children: "You are offline. Changes are kept locally." } };

export const Success: Story = { args: { tone: "success", children: "Chapter saved." } };

export const Info: Story = { args: { tone: "info", children: "A new revision is available." } };

export const Inline: Story = { args: { variant: "inline", tone: "error", children: "Title is required." } };

export const Dismissible: Story = {
  args: { tone: "info", onDismiss: () => {} },
  play: ({ canvasElement }) => {
    if (canvasElement.querySelector("[role=status]")?.querySelector("button[aria-label]") == null) throw new Error("dismiss button missing");
  },
};
