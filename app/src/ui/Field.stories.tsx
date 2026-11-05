import type { Meta, StoryObj } from "@storybook/react-vite";
import { Field } from "./Field";
import { TextField } from "./TextField";

const meta = {
  title: "UI/Field",
  component: Field,
  args: { label: "Title", children: <TextField placeholder="Untitled chapter" /> },
} satisfies Meta<typeof Field>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithTextField: Story = {
  name: "With text field",
  play: ({ canvasElement }) => {
    const label = canvasElement.querySelector("label")!;
    const input = canvasElement.querySelector("input")!;
    if (input.closest("label") !== label || label.textContent !== "Title") throw new Error("label is not wired to the input");
  },
};

export const WithSelect: Story = {
  name: "With select",
  args: { label: "Point of view", children: <TextField as="select"><option>First person</option><option>Third person</option></TextField> },
};
