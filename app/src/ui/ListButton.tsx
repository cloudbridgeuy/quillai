import type { ComponentProps } from "react";
import { cx } from "./cx";

export type ListButtonTone = "tree" | "lore";

type ListButtonProps = ComponentProps<"button"> & {
  pressed?: boolean;
  tone: ListButtonTone;
};

const tones: Record<ListButtonTone, string> = {
  tree: "rounded-card border-0 bg-transparent px-2 py-2 wrap-anywhere hover:bg-pi-pink focus-visible:bg-pi-pink aria-pressed:bg-pi-blue",
  lore: "grid grid-cols-[1fr_auto] gap-x-2 gap-y-1 rounded-card border border-rule bg-paper px-2 py-2 text-ink hover:bg-pi-lav focus-visible:bg-pi-lav",
};

export function listButtonClassName({ tone, className }: Pick<ListButtonProps, "tone" | "className">): string {
  return cx(
    "w-full cursor-pointer text-left transition-colors disabled:cursor-not-allowed disabled:opacity-50",
    tones[tone],
    className,
  );
}

export function ListButton({ pressed, tone, className, type = "button", ...props }: ListButtonProps) {
  return <button {...props} type={type} aria-pressed={pressed} className={listButtonClassName({ tone, className })} />;
}
