import type { ComponentProps } from "react";
import { cx } from "./cx";

export type AlertTone = "fail" | "inline";

type AlertProps = ComponentProps<"div"> & {
  tone: AlertTone;
};

const tones: Record<AlertTone, string> = {
  fail: "rounded-pill bg-alert px-4 py-2 text-label text-paper shadow-fail",
  inline: "text-ui text-alert",
};

export function alertClassName(tone: AlertTone, className?: string): string {
  return cx(tones[tone], className);
}

export function Alert({ tone, className, role = "alert", ...props }: AlertProps) {
  return <div {...props} role={role} className={alertClassName(tone, className)} />;
}
