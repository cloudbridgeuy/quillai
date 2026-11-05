import type { Meta, StoryObj } from "@storybook/react-vite";
import { Toggle } from "./Toggle";

const meta = {
  title: "UI/Toggle",
  component: Toggle,
  args: { label: "Show word count" },
} satisfies Meta<typeof Toggle>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Off: Story = {
  play: ({ canvasElement }) => {
    const input = canvasElement.querySelector("input")!;
    input.click();
    if (!input.checked) throw new Error("click did not check the toggle");
    input.click();
    if (input.checked) throw new Error("second click did not uncheck the toggle");
  },
};

export const On: Story = { args: { defaultChecked: true } };

export const Disabled: Story = { args: { disabled: true } };
