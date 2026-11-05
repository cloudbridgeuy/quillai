import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState, type ComponentProps } from "react";
import { Alert } from "./Alert";

const meta = {
  title: "UI/Alert",
  component: Alert,
  args: { tone: "error", children: "The chapter could not be saved." },
} satisfies Meta<typeof Alert>;

export default meta;
type Story = StoryObj<typeof meta>;

function DismissHarness(props: ComponentProps<typeof Alert>) {
  const [open, setOpen] = useState(true);
  return open ? <Alert {...props} onDismiss={() => setOpen(false)} /> : null;
}

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

export const DismissCallback: Story = {
  name: "Dismiss callback",
  args: { tone: "info" },
  render: (args) => <DismissHarness {...args} />,
  play: async ({ canvasElement }) => {
    canvasElement.querySelector<HTMLButtonElement>("[role=status] button[aria-label]")!.click();
    await new Promise((resolve) => setTimeout(resolve));
    if (canvasElement.querySelector("[role=status]") != null) throw new Error("onDismiss did not run");
  },
};
