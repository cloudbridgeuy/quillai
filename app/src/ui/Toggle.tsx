import type { ComponentProps } from "react";
import { cx } from "./cx";

type ToggleProps = Omit<ComponentProps<"input">, "type"> & {
  label: string;
};

export function toggleClassName(className?: string): string {
  return cx("inline-flex min-h-12 items-center gap-2 text-ui text-graphite", className);
}

export function Toggle({ label, className, ...props }: ToggleProps) {
  return (
    <label className={toggleClassName(className)}>
      <input {...props} type="checkbox" className="size-4" />
      <span>{label}</span>
    </label>
  );
}
