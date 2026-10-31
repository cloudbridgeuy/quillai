import { useState } from "react";
import { createRoot } from "react-dom/client";
import "../index.css";
import { Button } from "./Button";
import { IconButton } from "./IconButton";
import { ListButton } from "./ListButton";
import { Popover } from "./Popover";
import { TextField } from "./TextField";

function Fixture() {
  const [open, setOpen] = useState(false);
  const [icons, setIcons] = useState(0);
  const [lists, setLists] = useState(0);
  return <main>
    <Popover open={open} onOpenChange={setOpen} trigger={<Button>Open popover</Button>}>
      <TextField aria-label="Popover field" />
    </Popover>
    <Button>Focus button</Button>
    <TextField aria-label="Outside field" />
    <IconButton label="Count icon" glyph="+" onClick={() => setIcons(value => value + 1)} />
    <output data-testid="icon-count">{icons}</output>
    <ListButton tone="tree" onClick={() => setLists(value => value + 1)}>Count list</ListButton>
    <output data-testid="list-count">{lists}</output>
  </main>;
}

createRoot(document.getElementById("root")!).render(<Fixture />);
