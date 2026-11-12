import type { ComponentProps } from "react";
import { IconButton } from "../../ui/IconButton";
import { defaultLocale, formatMessage } from "../../i18n/messages";

type ContinueRailButtonProps = {
  saving: boolean;
  busy: boolean;
  cutOff: boolean;
  modelName: string | null;
  disabled: boolean;
  onMouseDown: NonNullable<ComponentProps<"button">["onMouseDown"]>;
  onClick: () => void;
  ref?: ComponentProps<"button">["ref"];
};

export function ContinueRailButton({ saving, busy, cutOff, modelName, disabled, onMouseDown, onClick, ref }: ContinueRailButtonProps) {
  const name = busy ? formatMessage(defaultLocale, "shortcut.stop") : formatMessage(defaultLocale, "editor.continue");
  const tooltipAction = saving ? formatMessage(defaultLocale, "action.saving") : name;
  const shortcut = formatMessage(defaultLocale, "shortcut.title", { action: tooltipAction, keys: "Ctrl/Cmd+Enter" });
  return (
    <IconButton
      ref={ref}
      kind="continue"
      className="editor-continue"
      label={name}
      aria-label={name}
      title={modelName ? formatMessage(defaultLocale, "editor.withModel", { title: shortcut, model: modelName }) : shortcut}
      glyph={<span aria-hidden="true">{busy ? "■" : "➤"}</span>}
      badge={cutOff ? <span className="editor-thread-indicator" role="img" aria-label={formatMessage(defaultLocale, "thread.resumeIndicator")} title={formatMessage(defaultLocale, "thread.resumeAction")} /> : null}
      aria-keyshortcuts="Control+Enter Meta+Enter"
      disabled={disabled}
      onMouseDown={onMouseDown}
      onClick={onClick}
    />
  );
}
