import type { ComponentProps, ReactNode } from "react";
import { cx } from "./cx";

type IconButtonProps = Omit<ComponentProps<"button">, "children"> & {
  label: string;
  glyph: ReactNode;
  badge?: ReactNode;
  pressed?: boolean;
  kind?: "continue";
};

export function iconButtonClassName({ className }: Pick<IconButtonProps, "className">): string {
  return cx(
    "relative shrink-0 size-9 rounded-icon border-0 bg-transparent text-title text-graphite transition-colors",
    "hover:bg-pi-pink hover:text-ink focus-visible:bg-pi-pink focus-visible:text-ink",
    "disabled:cursor-not-allowed disabled:opacity-50",
    "aria-pressed:bg-pi-green aria-pressed:text-ink",
    className,
  );
}

export function IconButton({ label, glyph, badge, pressed, kind, className, type = "button", title, ...props }: IconButtonProps) {
  return (
    <button {...props} type={type} aria-label={props["aria-label"] ?? label} title={title ?? (kind === "continue" ? label : undefined)} aria-pressed={pressed} className={iconButtonClassName({ className })}>
      {glyph}
      {badge ? <span className="absolute top-0 right-0">{badge}</span> : null}
    </button>
  );
}
