export type HintUnderlineProps = { top: number; left: number; width: number; height: number };

export function HintUnderline({ top, left, width, height }: HintUnderlineProps) {
  return <div data-hint-underline="" aria-hidden="true" className="absolute pointer-events-none border-b-2 border-slate" style={{ top, left, width, height, zIndex: "var(--z-hint)" }} />;
}
