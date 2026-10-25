import type { ComponentProps } from "react";
import { cx } from "./cx";

export type StickyTone = "yellow" | "pink" | "green";

type StickyNoteProps = ComponentProps<"textarea"> & {
  tone: StickyTone;
  overhang?: boolean;
};

export function stickyNoteClassName({ tone, overhang, className }: Pick<StickyNoteProps, "tone" | "overhang" | "className">): string {
  return cx("editor-sticky-note", `editor-sticky-note-${tone}`, overhang && "editor-sticky-note-overhang", className);
}

export function StickyNote({ tone, overhang, className, ...props }: StickyNoteProps) {
  return <textarea {...props} className={stickyNoteClassName({ tone, overhang, className })} />;
}
