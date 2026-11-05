import type { Meta, StoryObj } from "@storybook/react-vite";
import { Heading } from "./Heading";

const meta = {
  title: "UI/Heading",
  component: Heading,
  args: { level: "app", children: "The Long Winter" },
} satisfies Meta<typeof Heading>;

export default meta;
type Story = StoryObj<typeof meta>;

export const App: Story = {};

export const Section: Story = { args: { level: "section", children: "Chapters" } };

export const SectionAsH3: Story = {
  name: "Section as h3",
  args: { level: "section", as: "h3", children: "Lore" },
  play: ({ canvasElement }) => {
    if (canvasElement.querySelector("h3") == null) throw new Error("expected an h3");
  },
};
