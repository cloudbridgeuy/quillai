import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Select } from "./Select";

const options = [{ value: "", label: "No point (base text)" }, { value: "1", label: "Prologue" }, { value: "2", label: "Act two" }];

const meta = {
  title: "UI/Select",
  component: Select,
  args: { label: "This chapter happens at", value: "", options, onChange: () => {} },
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

function Echo() {
  const [value, setValue] = useState("");
  return (
    <div className="flex flex-col gap-2">
      <Select label="This chapter happens at" value={value} options={options} onChange={setValue} />
      <output>{value}</output>
    </div>
  );
}

export const Basic: Story = {
  render: () => <Echo />,
  play: async ({ canvasElement }) => {
    const select = canvasElement.querySelector("select")!;
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, "2");
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve));
    const echoed = canvasElement.querySelector("output")!.textContent;
    if (select.value !== "2" || echoed !== "2") throw new Error(`selection not reported: ${echoed}`);
  },
};

export const Disabled: Story = { args: { disabled: true } };
