import type { Meta, StoryObj } from "@storybook/react-vite";
import { StickyNote } from "./StickyNote";

const meta = {
  title: "UI/StickyNote",
  component: StickyNote,
  args: { tone: "yellow", placeholder: "Write a note", rows: 4 },
} satisfies Meta<typeof StickyNote>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Yellow: Story = {};

export const Pink: Story = { args: { tone: "pink", defaultValue: "Check the timeline in chapter 3." } };

export const Green: Story = { args: { tone: "green", defaultValue: "Resolved." } };

export const Overhang: Story = { args: { overhang: true } };

export const ReadOnly: Story = { name: "Read only", args: { readOnly: true, defaultValue: "This note is locked." } };

export const ReadOnlyContent: Story = {
  name: "Read only content",
  args: { readOnly: true, defaultValue: "This note is locked." },
  play: ({ canvasElement }) => {
    const note = canvasElement.querySelector("textarea");
    if (note == null) throw new Error("note did not render");
    if (note.value !== "This note is locked.") throw new Error("note content missing");
    if (!note.readOnly) throw new Error("note is not read only");
  },
};
