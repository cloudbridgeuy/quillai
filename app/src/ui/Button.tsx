import type { ComponentProps } from "react";
import { cx } from "./cx";

export type ButtonVariant = "quiet" | "primary" | "danger" | "link";
export type ButtonSize = "sm" | "md";

type ButtonProps = ComponentProps<"button"> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
};

const variants: Record<ButtonVariant, string> = {
  quiet: "border-rule bg-paper text-ink hover:border-graphite hover:bg-pi-pink",
  primary: "border-slate bg-slate text-paper hover:border-ink hover:bg-ink",
  danger: "border-alert bg-alert text-paper hover:opacity-90",
  link: "text-ink underline hover:text-graphite",
};

const sizes: Record<ButtonSize, string> = {
  sm: "min-h-9 px-3 py-1 text-label",
  md: "min-h-11 px-4 py-2 text-ui",
};

export function buttonClassName({ variant = "quiet", size = "md", className }: Pick<ButtonProps, "variant" | "size" | "className">): string {
  return cx(
    variant === "link" ? "font-semibold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-slate disabled:cursor-not-allowed disabled:opacity-50" : "border rounded-button font-semibold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-slate disabled:cursor-not-allowed disabled:opacity-50",
    variants[variant],
    variant !== "link" && sizes[size],
    className,
  );
}

export function Button({ variant, size, className, type = "button", ...props }: ButtonProps) {
  return <button {...props} type={type} className={buttonClassName({ variant, size, className })} />;
}
