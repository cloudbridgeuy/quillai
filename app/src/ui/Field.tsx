import type { ComponentProps, ReactNode } from "react";
import { cx } from "./cx";

type FieldProps = Omit<ComponentProps<"label">, "children"> & {
  label: string;
  children: ReactNode;
};

export function fieldClassName(className?: string): string {
  return cx("flex flex-col gap-2 text-label text-graphite", className);
}

export function Field({ label, children, className, ...props }: FieldProps) {
  return (
    <label {...props} className={fieldClassName(className)}>
      <span>{label}</span>
      {children}
    </label>
  );
}
