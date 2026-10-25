import type { ComponentProps } from "react";
import { cx } from "./cx";

export type HeadingLevel = "app" | "section";

type HeadingProps = ComponentProps<"h1"> & {
  level: HeadingLevel;
};

const levels: Record<HeadingLevel, string> = {
  app: "text-title font-semibold text-slate",
  section: "text-label font-semibold uppercase tracking-section text-slate",
};

export function headingClassName(level: HeadingLevel, className?: string): string {
  return cx("m-0", levels[level], className);
}

export function Heading({ level, className, ...props }: HeadingProps) {
  const Tag = level === "app" ? "h1" : "h2";
  return <Tag {...props} className={headingClassName(level, className)} />;
}
