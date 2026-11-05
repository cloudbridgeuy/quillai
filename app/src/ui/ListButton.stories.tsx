import { useState } from "react";
import type { ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { ListButton } from "./ListButton";

const meta = {
  title: "UI/ListButton",
  component: ListButton,
  args: { tone: "tree", children: "Chapter 1: The Thaw" },
} satisfies Meta<typeof ListButton>;

export default meta;
type Story = StoryObj<typeof meta>;

function ClickCounter(props: ComponentProps<typeof ListButton>) {
  const [clicks, setClicks] = useState(0);
  return (
    <div className="flex items-center gap-2">
      <ListButton {...props} onClick={() => setClicks(clicks + 1)} />
      <output>{clicks}</output>
    </div>
  );
}

export const Tree: Story = {
  render: (args) => <ClickCounter {...args} />,
  play: async ({ canvasElement }) => {
    canvasElement.querySelector("button")!.click();
    await new Promise((resolve) => setTimeout(resolve));
    const clicks = canvasElement.querySelector("output")!.textContent;
    if (clicks !== "1") throw new Error(`expected 1 click, got ${clicks}`);
  },
};

export const TreePressed: Story = { name: "Tree pressed", args: { pressed: true } };

export const Lore: Story = {
  args: { tone: "lore", children: <><span>Mara Quill</span><span className="text-meta text-graphite">Character</span></> },
};

export const Disabled: Story = { args: { disabled: true } };
