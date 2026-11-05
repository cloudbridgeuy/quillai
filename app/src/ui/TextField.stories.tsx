import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { TextField } from "./TextField";

const meta = {
  title: "UI/TextField",
  component: TextField,
  args: { placeholder: "Untitled chapter" },
} satisfies Meta<typeof TextField>;

export default meta;
type Story = StoryObj<typeof meta>;

function EchoField() {
  const [value, setValue] = useState("");
  return (
    <div className="flex flex-col gap-2">
      <TextField aria-label="Title" value={value} onChange={(event) => setValue(event.currentTarget.value)} />
      <output>{value}</output>
    </div>
  );
}

export const Input: Story = {
  render: () => <EchoField />,
  play: async ({ canvasElement }) => {
    const input = canvasElement.querySelector("input")!;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "Chapter one");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve));
    const echoed = canvasElement.querySelector("output")!.textContent;
    if (input.value !== "Chapter one" || echoed !== "Chapter one") throw new Error(`typed value not kept: ${echoed}`);
  },
};

export const Square: Story = { args: { square: true } };

export const Disabled: Story = { args: { disabled: true, defaultValue: "Locked title" } };

export const Select: Story = {
  render: () => <TextField as="select" aria-label="Status"><option>Draft</option><option>Final</option></TextField>,
};

export const Textarea: Story = {
  render: () => <TextField as="textarea" aria-label="Summary" rows={4} placeholder="Summary" />,
};
