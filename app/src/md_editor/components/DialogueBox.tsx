import { useState, type RefObject } from "react";

import { dialogueSpeakerOptions, MAX_DIALOGUE_SPEAKERS } from "../dialogue_request";
import type { DialogueChoice } from "../dialogue_stream";
import type { LoreMatchEntry } from "../lore_match";
import { Button } from "../../ui/Button";
import { IconButton } from "../../ui/IconButton";
import { Popover } from "../../ui/Popover";
import { StickyNote } from "../../ui/StickyNote";
import { Toggle } from "../../ui/Toggle";
import { defaultLocale, formatMessage, type MessageKey } from "../../i18n/messages";

const t = (key: MessageKey) => formatMessage(defaultLocale, key);

export function DialogueBox({ loreEntries, disabled, open, onOpenChange, onWrite, ignoreRefs }: {
  loreEntries: readonly LoreMatchEntry[];
  disabled: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onWrite: (choice: DialogueChoice) => void;
  ignoreRefs?: readonly RefObject<HTMLElement | null>[];
}) {
  const [chosen, setChosen] = useState<string[]>([]);
  const [brief, setBrief] = useState("");
  const options = dialogueSpeakerOptions(loreEntries);
  const speakers = chosen.filter(name => options.some(option => option.name === name));
  const canWrite = !disabled && speakers.length > 0;
  const write = () => { if (canWrite) onWrite({ speakers, brief }); };

  return <Popover anchor={{ left: 0, top: 0 }} ignoreRefs={ignoreRefs} open={open} onOpenChange={onOpenChange}
    className="editor-edit-box" role="dialog" aria-label={t("editor.dialogue")}
    trigger={<IconButton label={t("editor.dialogue")} glyph={<span aria-hidden="true">❝</span>} title={t("editor.dialogue")}
      onMouseDown={(event) => event.preventDefault()} />}>
    <div className="editor-edit-flag editor-dialogue-flag">{t("editor.dialogue")}</div>
    <fieldset className="flex flex-col">
      <legend className="editor-note">{t("editor.dialogueSpeakers")}</legend>
      {options.length === 0 ? <p className="editor-note">{t("editor.dialogueNoSpeakers")}</p> : null}
      {options.map(option => <Toggle key={option.id} label={option.name} checked={speakers.includes(option.name)}
        disabled={!speakers.includes(option.name) && speakers.length >= MAX_DIALOGUE_SPEAKERS}
        onChange={(event) => setChosen(event.target.checked ? [...speakers, option.name] : speakers.filter(name => name !== option.name))} />)}
    </fieldset>
    <div className="-mx-4">
      <StickyNote tone="yellow" overhang rows={3} aria-label={t("editor.dialogueBrief")} aria-describedby="dialogue-brief-hint"
        placeholder={t("editor.dialoguePlaceholder")} value={brief} disabled={disabled}
        onChange={(event) => setBrief(event.target.value)}
        onKeyDown={(event) => {
          if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
          event.preventDefault();
          write();
        }} />
    </div>
    <p id="dialogue-brief-hint" className="editor-note">{t("editor.dialogueHint")}</p>
    <div className="editor-actions">
      <Button variant="primary" size="sm" disabled={!canWrite} onClick={write}>{t("editor.dialogueWrite")}</Button>
    </div>
  </Popover>;
}
