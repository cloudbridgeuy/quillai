import { useState } from "react";
import type { ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "./Button";

const meta = {
  title: "UI/Button",
  component: Button,
  args: { children: "Save", variant: "quiet", size: "md" },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

function ClickCounter(props: ComponentProps<typeof Button>) {
  const [clicks, setClicks] = useState(0);
  return (
    <div className="flex items-center gap-2">
      <Button {...props} onClick={() => setClicks(clicks + 1)} />
      <output>{clicks}</output>
    </div>
  );
}

async function clickAndCount(canvas: HTMLElement): Promise<string> {
  canvas.querySelector("button")!.click();
  await new Promise((resolve) => setTimeout(resolve));
  return canvas.querySelector("output")!.textContent ?? "";
}

export const Quiet: Story = {};

export const Primary: Story = {
  args: { variant: "primary" },
  render: (args) => <ClickCounter {...args} />,
  play: async ({ canvasElement }) => {
    const clicks = await clickAndCount(canvasElement);
    if (clicks !== "1") throw new Error(`expected 1 click, got ${clicks}`);
  },
};

export const Danger: Story = { args: { variant: "danger", children: "Delete" } };

export const Link: Story = { args: { variant: "link", children: "Open chapter" } };

export const Small: Story = { args: { size: "sm" } };

export const Disabled: Story = {
  args: { variant: "primary", disabled: true },
  render: (args) => <ClickCounter {...args} />,
  play: async ({ canvasElement }) => {
    const clicks = await clickAndCount(canvasElement);
    if (clicks !== "0") throw new Error(`disabled button counted ${clicks} clicks`);
  },
};
