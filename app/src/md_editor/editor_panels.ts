export type EditorPanel = "manuscript" | "context" | "settings";
export type PanelState = EditorPanel | null;
export type PanelAction = EditorPanel | "close";

export function transitionPanel(state: PanelState, action: PanelAction): PanelState {
  return action === "close" || state === action ? null : action;
}

export function panelShortcut(event: {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
  isComposing: boolean;
}): PanelAction | null {
  if (event.isComposing || event.repeat) return null;
  if (event.key === "Escape") return "close";
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null;
  if (event.key === "[") return "manuscript";
  if (event.key === "]") return "context";
  return null;
}
