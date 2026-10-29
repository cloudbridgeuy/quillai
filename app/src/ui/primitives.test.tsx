import { describe, expect, test } from "bun:test";
import { createRef } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Alert, alertClassName } from "./Alert";
import { Button, buttonClassName } from "./Button";
import { Field } from "./Field";
import { Heading, headingClassName } from "./Heading";
import { IconButton, iconButtonClassName } from "./IconButton";
import { ListButton, listButtonClassName } from "./ListButton";
import { Modal, modalClassName, modalReturnFocus } from "./Modal";
import { Popover, popoverClassName } from "./Popover";
import { StickyNote, stickyNoteClassName } from "./StickyNote";
import { TextField, textFieldClassName } from "./TextField";
import { Toggle } from "./Toggle";

describe("Button", () => {
  test("variants and sizes", () => {
    expect(buttonClassName({})).toContain("bg-paper");
    expect(buttonClassName({})).toContain("min-h-11");
    expect(buttonClassName({ variant: "primary" })).toContain("bg-slate");
    expect(buttonClassName({ variant: "danger", size: "sm" })).toContain("bg-alert");
    expect(buttonClassName({ size: "sm" })).toContain("min-h-9");
    expect(buttonClassName({ className: "extra" })).toEndWith("extra");
  });

  test("forwards props, defaults type and keeps disabled", () => {
    const html = renderToStaticMarkup(<Button disabled aria-label="x" data-k="1">Go</Button>);
    expect(html).toContain('type="button"');
    expect(html).toContain("disabled");
    expect(html).toContain('data-k="1"');
  });

  test("forwards ref", () => {
    const ref = createRef<HTMLButtonElement>();
    expect(Button({ ref, children: "a" }).props.ref).toBe(ref);
  });
});

describe("IconButton", () => {
  test("kinds", () => {
    expect(iconButtonClassName({})).toContain("size-9");
    expect(iconButtonClassName({ kind: "continue" })).toContain("w-14");
  });

  test("label and pressed", () => {
    const html = renderToStaticMarkup(<IconButton label="Open" glyph="+" pressed />);
    expect(html).toContain('aria-label="Open"');
    expect(html).toContain('aria-pressed="true"');
    expect(renderToStaticMarkup(<IconButton label="Open" glyph="+" kind="continue" />)).toContain("<span");
  });
});

describe("ListButton", () => {
  test("tones and className", () => {
    expect(listButtonClassName({ tone: "tree" })).toContain("aria-pressed:bg-pi-blue");
    expect(listButtonClassName({ tone: "lore" })).toContain("border-rule");
    expect(listButtonClassName({ tone: "lore", className: "extra" })).toEndWith("extra");
  });

  test("pressed, type and forwarded props", () => {
    const html = renderToStaticMarkup(<ListButton tone="tree" pressed disabled data-k="1">Ch</ListButton>);
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain('type="button"');
    expect(html).toContain("disabled");
    expect(html).toContain('data-k="1"');
  });
});

describe("TextField", () => {
  test("radius", () => {
    expect(textFieldClassName({})).toContain("rounded-card");
    expect(textFieldClassName({ square: true })).toContain("rounded-none");
  });

  test("as select and input forward props", () => {
    expect(renderToStaticMarkup(<TextField as="select" disabled aria-label="m"><option>a</option></TextField>)).toStartWith("<select");
    const html = renderToStaticMarkup(<TextField readOnly placeholder="p" />);
    expect(html).toStartWith("<input");
    expect(html).toContain("readOnly");
    expect(html).not.toContain("as=");
  });
});

describe("Toggle and Field", () => {
  test("toggle renders checkbox with label", () => {
    const html = renderToStaticMarkup(<Toggle label="Tint" checked readOnly />);
    expect(html).toContain('type="checkbox"');
    expect(html).toContain("<span>Tint</span>");
  });

  test("field wraps control", () => {
    const html = renderToStaticMarkup(<Field label="Name"><TextField /></Field>);
    expect(html).toContain("<span>Name</span>");
    expect(html).toContain("text-graphite");
  });
});

describe("Heading", () => {
  test("levels", () => {
    expect(headingClassName("app")).toContain("text-title");
    expect(headingClassName("section")).toContain("uppercase");
    expect(renderToStaticMarkup(<Heading level="app">A</Heading>)).toStartWith("<h1");
    expect(renderToStaticMarkup(<Heading level="section">A</Heading>)).toStartWith("<h2");
  });
});

describe("Alert", () => {
  test("tones and role", () => {
    expect(alertClassName("fail")).toContain("shadow-fail");
    expect(alertClassName("inline")).toContain("text-alert");
    expect(renderToStaticMarkup(<Alert tone="inline">x</Alert>)).toContain('role="alert"');
    expect(renderToStaticMarkup(<Alert tone="fail" role="status">x</Alert>)).toContain('role="status"');
  });
});

describe("StickyNote", () => {
  test("tones keep existing classes", () => {
    expect(stickyNoteClassName({ tone: "yellow" })).toBe("editor-sticky-note editor-sticky-note-yellow");
    expect(stickyNoteClassName({ tone: "green" })).toContain("editor-sticky-note-green");
    expect(stickyNoteClassName({ tone: "pink", overhang: true })).toContain("editor-sticky-note-overhang");
  });

  test("readOnly and native props pass through", () => {
    const html = renderToStaticMarkup(<StickyNote tone="pink" readOnly rows={3} aria-label="n" value="v" />);
    expect(html).toContain("readOnly");
    expect(html).toContain('rows="3"');
  });
});

describe("Modal", () => {
  test("class and title semantics", () => {
    expect(modalClassName("extra")).toEndWith("extra");
    const html = renderToStaticMarkup(<Modal title="Edit" onClose={() => {}} returnFocusRef={createRef<HTMLButtonElement>()}>Body</Modal>);
    expect(html).toContain("<dialog");
    expect(html).toContain("aria-labelledby=");
    expect(html).toContain(">Edit</h2>");
    expect(html).not.toContain("<footer");
    expect(html).not.toContain("returnFocusRef");
  });

  test("return focus prefers explicit target and ignores body fallback", () => {
    const button = { tagName: "BUTTON", focus() {} };
    const other = { tagName: "INPUT", focus() {} };
    const body = { tagName: "BODY", focus() {} };
    expect(modalReturnFocus(button, body)).toBe(button);
    expect(modalReturnFocus(button, other)).toBe(button);
    expect(modalReturnFocus(null, other)).toBe(other);
    expect(modalReturnFocus(null, body)).toBeNull();
    expect(modalReturnFocus(null, null)).toBeNull();
  });

  test("Modal footer renders only its children", () => {
    const html = renderToStaticMarkup(<Modal title="Edit" onClose={() => {}} footer={<Button>Save</Button>}>Body</Modal>);
    expect(html.match(/<footer[^>]*>(.*?)<\/footer>/)?.[1]).toBe(renderToStaticMarkup(<Button>Save</Button>));
    expect(html).not.toContain("Cancel");
    expect(html).not.toContain("Close");
  });
});

describe("Popover", () => {
  test("class and trigger semantics", () => {
    expect(popoverClassName("extra")).toEndWith("extra");
    const html = renderToStaticMarkup(<Popover trigger={<Button>Open</Button>} open onOpenChange={() => {}}>Content</Popover>);
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("aria-controls=");
    expect(html).toContain(">Content</div>");
    const closed = renderToStaticMarkup(<Popover trigger={<Button>Open</Button>} open={false} onOpenChange={() => {}}>Content</Popover>);
    expect(closed).toContain('aria-expanded="false"');
    expect(closed).not.toContain("Content");
  });
});
