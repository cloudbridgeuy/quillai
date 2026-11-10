import type { Meta, StoryObj } from "@storybook/react-vite";
import { HintUnderline } from "./HintUnderline";

const meta = {
  title: "UI/HintUnderline",
  component: HintUnderline,
  args: { top: 0, left: 0, width: 120, height: 24 },
  decorators: [(Story) => <div className="relative h-8 w-48 text-ink"><span>a hinted phrase</span><Story /></div>],
} satisfies Meta<typeof HintUnderline>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Narrow: Story = { args: { left: 16, width: 48 } };
