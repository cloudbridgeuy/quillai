import type { ComponentProps, ReactNode } from "react";
import { cx } from "./cx";

type IconButtonProps = Omit<ComponentProps<"button">, "children"> & {
  label: string;
  glyph: ReactNode;
  pressed?: boolean;
  kind?: "continue";
};

export function iconButtonClassName({ kind, className }: Pick<IconButtonProps, "kind" | "className">): string {
  return cx(
    "shrink-0 rounded-icon border-0 bg-transparent text-graphite transition-colors",
    "hover:bg-pi-pink hover:text-ink focus-visible:bg-pi-pink focus-visible:text-ink",
    "disabled:cursor-not-allowed disabled:opacity-50",
    "aria-pressed:bg-pi-green aria-pressed:text-ink",
    kind === "continue" ? "flex min-h-12 w-14 flex-col items-center gap-0.5 py-0.5 text-title" : "size-9 text-title",
    className,
  );
}

export function IconButton({ label, glyph, pressed, kind, className, type = "button", ...props }: IconButtonProps) {
  return (
    <button {...props} type={type} aria-label={label} aria-pressed={pressed} className={iconButtonClassName({ kind, className })}>
      {glyph}
      {kind === "continue" ? <span className="text-micro">{label}</span> : null}
    </button>
  );
}
