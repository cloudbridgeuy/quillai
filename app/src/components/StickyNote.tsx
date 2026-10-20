import type { ComponentProps } from "react";

type StickyNoteProps = ComponentProps<"textarea"> & {
  tone: "yellow" | "pink";
};

export function StickyNote({ tone, className = "", ...props }: StickyNoteProps) {
  return <textarea {...props} className={`editor-sticky-note editor-sticky-note-${tone} ${className}`} />;
}
