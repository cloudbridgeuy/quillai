import { createRoot } from "react-dom/client";
import "../../../app/src/index.css";
import { Alert } from "../../../app/src/ui/Alert";
import { Button } from "../../../app/src/ui/Button";
import { Field } from "../../../app/src/ui/Field";
import { Heading } from "../../../app/src/ui/Heading";
import { IconButton } from "../../../app/src/ui/IconButton";
import { ListButton } from "../../../app/src/ui/ListButton";
import { StickyNote } from "../../../app/src/ui/StickyNote";
import { TextField } from "../../../app/src/ui/TextField";
import { Toggle } from "../../../app/src/ui/Toggle";

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section data-testid={id} className="flex flex-col gap-3 p-4">
      <Heading level="section">{title}</Heading>
      <div className="flex flex-wrap items-center gap-3">{children}</div>
    </section>
  );
}

function Page() {
  return (
    <main className="bg-desk p-6 text-ink">
      <Section id="heading" title="Heading">
        <Heading level="app">QuillAI app heading</Heading>
        <Heading level="section">Section heading</Heading>
      </Section>
      <Section id="button" title="Button">
        {(["quiet", "primary", "danger"] as const).flatMap((variant) =>
          (["md", "sm"] as const).flatMap((size) => [
            <Button key={`${variant}-${size}`} variant={variant} size={size}>{variant} {size}</Button>,
            <Button key={`${variant}-${size}-d`} variant={variant} size={size} disabled>{variant} {size} disabled</Button>,
          ]),
        )}
      </Section>
      <Section id="icon-button" title="IconButton">
        <IconButton label="Manuscript" glyph="M" pressed={false} />
        <IconButton label="Context" glyph="C" pressed />
        <IconButton label="Disabled" glyph="D" disabled />
        <IconButton label="Continue" glyph="+" kind="continue" />
        <IconButton label="Continue pressed" glyph="+" kind="continue" pressed />
      </Section>
      <Section id="list-button" title="ListButton">
        <ul className="m-0 flex w-64 list-none flex-col gap-1 p-0">
          <li><ListButton tone="tree">Tree default</ListButton></li>
          <li><ListButton tone="tree" pressed>Tree pressed</ListButton></li>
          <li><ListButton tone="tree" disabled>Tree disabled</ListButton></li>
        </ul>
        <ul className="editor-lore-window m-0 w-64 list-none p-0">
          <li>
            <ListButton tone="lore">
              <strong>Mira</strong><span>3 matches</span>
              <small>character</small><small>include</small>
            </ListButton>
          </li>
        </ul>
      </Section>
      <Section id="text-field" title="TextField">
        <TextField aria-label="Input" placeholder="Input" />
        <TextField aria-label="Square input" placeholder="Square" square />
        <TextField aria-label="Disabled input" placeholder="Disabled" disabled />
        <TextField as="select" aria-label="Select"><option>One</option><option>Two</option></TextField>
        <TextField as="select" aria-label="Square select" square><option>One</option></TextField>
      </Section>
      <Section id="toggle" title="Toggle">
        <Toggle label="Unchecked" />
        <Toggle label="Checked" defaultChecked />
        <Toggle label="Disabled" disabled />
      </Section>
      <Section id="field" title="Field">
        <Field label="Title"><TextField placeholder="Chapter title" /></Field>
        <Field label="Model"><TextField as="select"><option>Fast</option></TextField></Field>
      </Section>
      <Section id="alert" title="Alert">
        <Alert tone="fail">Not saved</Alert>
        <Alert tone="inline">Could not load lore</Alert>
      </Section>
      <Section id="sticky-note" title="StickyNote">
        <StickyNote tone="yellow" aria-label="Yellow note" defaultValue="Yellow" rows={2} />
        <StickyNote tone="pink" aria-label="Pink note" defaultValue="Pink" rows={2} />
        <StickyNote tone="green" aria-label="Green note" defaultValue="Green" rows={2} />
        <StickyNote tone="yellow" aria-label="Overhang note" defaultValue="Overhang" rows={2} overhang />
        <StickyNote tone="green" aria-label="Read-only note" defaultValue="Read-only" rows={2} readOnly />
        <StickyNote tone="pink" aria-label="Disabled note" defaultValue="Disabled" rows={2} disabled />
      </Section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Page />);
