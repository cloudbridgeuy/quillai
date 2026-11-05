import { useState, type ReactNode } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "./Button";
import { Modal } from "./Modal";

const meta = {
  title: "UI/Modal",
  component: Modal,
  args: { title: "Delete chapter?", children: "This removes the chapter and its notes.", onClose: () => {} },
} satisfies Meta<typeof Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

function ModalOpener({ title, children }: { title: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const close = () => setOpen(false);
  return (
    <>
      <Button onClick={() => setOpen(true)}>Open</Button>
      {open ? <Modal title={title} onClose={close} footer={<Button variant="primary" onClick={close}>Close</Button>}>{children}</Modal> : null}
    </>
  );
}

const settle = () => new Promise((resolve) => setTimeout(resolve));

export const Open: Story = {
  render: (args) => <Modal {...args} footer={<><Button>Cancel</Button><Button variant="danger">Delete</Button></>} />,
};

export const OpenAndClose: Story = {
  name: "Open and close",
  render: (args) => <ModalOpener title={args.title}>{args.children}</ModalOpener>,
  play: async ({ canvasElement }) => {
    const opener = canvasElement.querySelector("button")!;
    opener.focus();
    opener.click();
    await settle();
    const dialog = canvasElement.querySelector("dialog");
    if (!dialog?.open) throw new Error("modal did not open");
    [...dialog.querySelectorAll("button")].find((button) => button.textContent === "Close")!.click();
    await settle();
    if (canvasElement.querySelector("dialog")) throw new Error("modal did not close");
    if (document.activeElement !== opener) throw new Error("focus did not return to the opener");
  },
};
