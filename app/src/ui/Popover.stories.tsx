import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "./Button";
import { Popover } from "./Popover";

const meta = {
  title: "UI/Popover",
  component: Popover,
  args: { open: false, onOpenChange: () => {}, children: "Rename, move, or delete this chapter.", trigger: <Button>Options</Button> },
} satisfies Meta<typeof Popover>;

export default meta;
type Story = StoryObj<typeof meta>;

const settle = () => new Promise((resolve) => setTimeout(resolve));

function TriggeredPopover() {
  const [open, setOpen] = useState(false);
  return <Popover open={open} onOpenChange={setOpen} trigger={<Button>Options</Button>}>Rename, move, or delete this chapter.</Popover>;
}

function AnchoredPopover() {
  const [open, setOpen] = useState(true);
  return <Popover open={open} onOpenChange={setOpen} anchor={{ left: 48, top: 48 }}>Anchored at a caret position.</Popover>;
}

export const Triggered: Story = {
  render: () => <TriggeredPopover />,
  play: async ({ canvasElement }) => {
    const trigger = canvasElement.querySelector("button")!;
    trigger.click();
    await settle();
    if (trigger.getAttribute("aria-expanded") !== "true" || document.getElementById(trigger.getAttribute("aria-controls")!) == null) throw new Error("popover did not open");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await settle();
    if (trigger.getAttribute("aria-expanded") !== "false") throw new Error("Escape did not close the popover");
    if (document.activeElement !== trigger) throw new Error("focus did not return to the trigger");
    trigger.click();
    await settle();
    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await settle();
    if (trigger.getAttribute("aria-expanded") !== "false") throw new Error("outside pointerdown did not close the popover");
  },
};

export const Open: Story = { args: { open: true } };

export const Anchored: Story = { render: () => <AnchoredPopover /> };
