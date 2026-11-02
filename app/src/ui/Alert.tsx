import type { ComponentProps } from "react";
import { cx } from "./cx";
import { IconButton } from "./IconButton";

export type AlertTone = "error" | "warning" | "success" | "info";
type AlertVariant = "pill" | "inline";

type AlertProps = ComponentProps<"div"> & {
  tone: AlertTone;
  variant?: AlertVariant;
  onDismiss?: () => void;
};

const fills: Record<AlertTone, string> = {
  error: "bg-alert text-paper shadow-fail",
  warning: "bg-pi-yellow text-ink",
  success: "bg-pi-green text-ink",
  info: "bg-pi-blue text-ink",
};

const markers: Record<AlertTone, string> = {
  error: "bg-alert",
  warning: "bg-pi-yellow",
  success: "bg-pi-green",
  info: "bg-pi-blue",
};

export function alertClassName(tone: AlertTone, variant: AlertVariant = "pill", className?: string): string {
  return cx(variant === "pill" ? `rounded-pill px-4 py-2 text-label ${fills[tone]}` : `text-ui ${tone === "error" ? "text-alert" : "text-ink"}`, className);
}

export function Alert({ tone, variant = "pill", onDismiss, className, role, children, ...props }: AlertProps) {
  return <div {...props} data-tone={tone} role={role ?? (tone === "error" ? "alert" : "status")} className={alertClassName(tone, variant, className)}>
    {variant === "inline" ? <span aria-hidden="true" className={cx("mr-2 inline-block size-2 rounded-full", markers[tone])} /> : null}
    {children}
    {onDismiss ? <IconButton label="Dismiss" glyph="×" onClick={onDismiss} /> : null}
  </div>;
}
