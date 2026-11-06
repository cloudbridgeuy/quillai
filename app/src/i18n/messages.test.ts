import { expect, test } from "bun:test";
import * as messages from "./messages";
import { en, es, formatMessage, type MessageCatalog } from "./messages";
import { formatTextCounts } from "../md_editor/text_counts";
import { getSavePresentation } from "../md_editor/editor_ai";

test("plural forms in english and spanish", () => {
  expect(formatMessage("en", "chapter.count", { count: 0 })).toBe("0 chapters");
  expect(formatMessage("en", "chapter.count", { count: 1 })).toBe("1 chapter");
  expect(formatMessage("en", "chapter.count", { count: 2 })).toBe("2 chapters");

  expect(formatMessage("es", "chapter.count", { count: 0 })).toBe("0 capítulos");
  expect(formatMessage("es", "chapter.count", { count: 1 })).toBe("1 capítulo");
  expect(formatMessage("es", "chapter.count", { count: 2 })).toBe("2 capítulos");
});

test("a missing plural category falls back to other", () => {
  expect(formatMessage("es", "chapter.count", { count: 1_000_000 })).toBe("1.000.000 capítulos");
  expect(formatMessage("en", "chapter.count")).toBe("{count} chapters");
  expect(formatMessage("en", "chapter.count", { count: Number.NaN })).toBe("NaN chapters");
});

test("interpolation inserts strings and leaves unknown placeholders", () => {
  expect(formatMessage("en", "greeting.hello", { name: "Ada" })).toBe("Hello, Ada");
  expect(formatMessage("es", "greeting.hello", { name: "Ada" })).toBe("Hola, Ada");
  expect(formatMessage("en", "greeting.hello")).toBe("Hello, {name}");
  expect(formatMessage("en", "greeting.hello", { name: "a {name}" })).toBe("Hello, a {name}");
});

test("number formatting follows the locale", () => {
  expect(formatMessage("en", "draft.words", { count: 1234.5 })).toBe("1,234.5 words");
  expect(formatMessage("es", "draft.words", { count: 1234.5 })).toBe("1234,5 palabras");
});

test("dates and currency use Intl", () => {
  const when = new Date(Date.UTC(2026, 9, 22));
  const price = { amount: 1234.5, currency: "USD" };
  expect(formatMessage("en", "edit.summary", { when, price })).toBe("Edited Oct 22, 2026 for $1,234.50");
  expect(formatMessage("es", "edit.summary", { when, price })).toBe("Editado el 22 oct 2026 por 1234,50\u00a0US$");
});

test("missing key falls back to english, then to the key", () => {
  const catalog: MessageCatalog = {
    en,
    es: { ...es, "action.save": undefined },
  };
  expect(formatMessage("es", "action.save", undefined, catalog)).toBe("Save");
  expect(formatMessage("es", "action.save")).toBe("Guardar");
  expect(formatMessage("en", "no.such.key")).toBe("no.such.key");
  expect(formatMessage("es", "no.such.key")).toBe("no.such.key");

  const words: MessageCatalog = {
    en,
    es: { ...es, "draft.words": undefined },
  };
  expect(formatMessage("es", "draft.words", { count: 1234.5 }, words)).toBe("1234,5 words");
});

test("a bad currency code throws", () => {
  const when = new Date(Date.UTC(2026, 9, 22));
  expect(() => formatMessage("en", "edit.summary", { when, price: { amount: 1, currency: "nope" } })).toThrow();
});

test("regional spanish tags use the es table and es formatting", () => {
  expect(formatMessage("es-MX", "draft.words", { count: 1234.5 })).toBe("1234,5 palabras");
  expect(formatMessage("es-ES", "action.save")).toBe("Guardar");
  expect(formatMessage("es-419", "action.save")).toBe("Guardar");
  expect(formatMessage("ES", "action.save")).toBe("Guardar");
});

test("unknown locale uses the english table and english number format", () => {
  expect(formatMessage("fr", "action.save")).toBe("Save");
  expect(formatMessage("en-US", "draft.words", { count: 1234.5 })).toBe("1,234.5 words");
});

test("spanish table has the same keys as english", () => {
  expect(Object.keys(es).sort()).toEqual(Object.keys(en).sort());
  for (const key of Object.keys(en) as (keyof typeof en)[]) {
    expect(typeof es[key]).toBe(typeof en[key]);
  }
});

test.each([
  ["app.loading", "Loading…"],
  ["app.name", "QuillAI"],
  ["error.unexpected", "Something went wrong: {message}"],
  ["error.retry", "Try again"],
  ["landing.activity.title", "Recent activity"],
  ["landing.activity.loadFailed", "Could not load recent activity."],
  ["landing.open", "Open"],
  ["landing.checking", "Checking…"],
  ["landing.checkFailed", "Could not check it: {message}"],
  ["landing.forget", "Forget"],
  ["landing.empty", "Nothing edited in this browser yet. Open or create a piece below."],
  ["landing.yourLibrary", "Your library"],
  ["landing.resume", "Resume"],
  ["landing.library", "Library"],
  ["landing.tips", "Tips"],
  ["alert.dismiss", "Dismiss"],
  ["landing.deleted.title", "Recently deleted"],
  ["landing.deleted.loadFailed", "Could not load deleted items"],
  ["landing.deleted.loadFailedEmpty", "Could not load deleted items."],
  ["landing.deleted.actionFailed", "Action failed"],
  ["landing.deleted.retry", "Retry"],
  ["landing.deleted.empty", "No recently deleted items."],
  ["landing.deleted.restoreNotice", "Restoring this item also restores: {parents}."],
  ["landing.deleted.deletePrompt", "Delete {title} forever?"],
  ["landing.deleted.deleteWithCounts", "Delete {title} forever with {counts}?"],
  ["landing.deleted.deleteWithParents", "Delete {title} forever? This also restores: {parents}."],
  ["landing.deleted.deleteWithCountsAndParents", "Delete {title} forever with {counts}? This also restores: {parents}."],
  ["landing.deleted.confirmRestore", "Confirm restore"],
  ["landing.deleted.restore", "Restore"],
  ["landing.deleted.confirmDelete", "Confirm delete"],
  ["landing.deleted.deleteForever", "Delete forever"],
  ["landing.deleted.chapters", "{countText} chapters"],
  ["landing.deleted.pieces", "{countText} pieces"],
  ["landing.deleted.lore", "{countText} lore"],
  ["settings.ai.loadFailed", "Could not load AI settings."],
  ["settings.ai.saveFailed", "Could not save AI settings."],
  ["settings.ai.savedHere", "Saved here"],
  ["settings.ai.fromEnv", "From quillai.env"],
  ["settings.ai.notSet", "Not set"],
  ["settings.ai.title", "AI"],
  ["settings.ai.baseUrl", "Base URL"],
  ["settings.ai.model", "Model"],
  ["settings.ai.keySaved", "Key saved"],
  ["settings.ai.noKey", "No key set"],
  ["settings.ai.replace", "Replace"],
  ["settings.ai.remove", "Remove"],
  ["settings.ai.replacementKey", "Replacement API key"],
  ["settings.ai.save", "Save AI settings"],
])("UI message %s preserves exact English and has Spanish", (key, english) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key)).toBe(english);
});

test("UI placeholders preserve English text and raw counts", () => {
  expect(formatMessage("en", "error.unexpected", { message: "offline" })).toBe("Something went wrong: offline");
  expect(formatMessage("en", "landing.checkFailed", { message: "offline" })).toBe("Could not check it: offline");
  expect(formatMessage("en", "landing.deleted.restoreNotice", { parents: "Book, Series" })).toBe("Restoring this item also restores: Book, Series.");
  expect(formatMessage("en", "landing.deleted.deleteWithCountsAndParents", { title: "Draft", counts: "1000 chapters", parents: "Book" })).toBe("Delete Draft forever with 1000 chapters? This also restores: Book.");
  expect(formatMessage("en", "landing.deleted.chapters", { count: 1000, countText: "1000" })).toBe("1000 chapters");
  expect(formatMessage("en", "landing.deleted.chapters", { count: 1, countText: "1" })).toBe("1 chapters");
  expect(formatMessage("es", "landing.deleted.chapters", { count: 1, countText: "1" })).toBe("1 capítulo");
  expect(formatMessage("es", "landing.deleted.pieces", { count: 2, countText: "2" })).toBe("2 textos");
});

test.each([
  ["tree.renameFailed", "Could not rename item."], ["tree.rename", "Rename '{title}'"],
  ["tree.delete", "Delete '{title}'"], ["tree.actionFailed", "Action failed."],
  ["tree.returnFailed", "Item deleted, but returning to the library failed: {message} Use Undo to restore it."], ["tree.restoreFailed", "Could not restore item."],
  ["tree.createFailed", "Could not create manuscript item."], ["tree.kind", "Kind"],
  ["tree.label", "Label"], ["tree.format", "Format"],
  ["tree.script", "Script"], ["tree.prose", "Prose"],
  ["tree.create", "Create"], ["tree.cancel", "Cancel"],
  ["tree.loadFailed", "Could not load manuscript."], ["tree.loading", "Loading manuscript…"],
  ["tree.retry", "Try loading again"], ["tree.noScenes", "No scenes yet."],
  ["tree.noChapters", "No chapters yet."], ["tree.deleted", "Deleted '{title}'."],
  ["tree.undo", "Undo"], ["tree.noUniverses", "No universes yet."],
  ["tree.noWorks", "No works yet."], ["tree.noPieces", "No pieces yet."],
  ["tree.create.loosePiece", "Loose piece"], ["tree.create.chapter", "Chapter"],
  ["tree.create.chapter.title", "Chapter title"], ["tree.create.chapter.creating", "Creating chapter…"],
  ["tree.create.scene", "Scene"], ["tree.create.scene.title", "Scene title"],
  ["tree.create.scene.creating", "Creating scene…"], ["tree.create.universe", "Universe"],
  ["tree.create.universe.title", "Universe title"], ["tree.create.universe.creating", "Creating universe…"],
  ["tree.create.container", "Container"], ["tree.create.container.title", "Container title"],
  ["tree.create.container.creating", "Creating container…"], ["tree.create.piece", "Piece"],
  ["tree.create.piece.title", "Piece title"], ["tree.create.piece.creating", "Creating piece…"],
  ["tree.create.item", "Item"], ["tree.create.item.title", "Item title"],
  ["tree.create.item.creating", "Creating item…"], ["lore.saveFailed", "Could not save lore."],
  ["timeline.open", "Timeline"], ["timeline.title", "Timeline"], ["timeline.empty", "No timeline points yet."],
  ["timeline.name", "Point name"], ["timeline.add", "Add point"], ["timeline.rename", "Rename point"],
  ["timeline.delete", "Delete point"], ["timeline.new", "New point"], ["timeline.failed", "Could not update the timeline."],
  ["timeline.chapterAt", "This chapter happens at"], ["timeline.base", "No point (base text)"],
  ["lore.series", "Series lore"], ["lore.loading", "Loading lore…"],
  ["lore.retry", "Retry lore"], ["lore.empty", "No lore yet."],
  ["lore.pinned", " · pinned"], ["lore.new", "New lore"],
  ["lore.create", "Create lore"], ["lore.edit", "Edit lore"],
  ["lore.delete", "Delete lore"], ["lore.save", "Save lore"],
  ["lore.name", "Name"], ["lore.keywords", "Keywords (one per line)"],
  ["lore.aliases", "Aliases (one per line)"], ["lore.visible", "Visible text (AI context)"],
  ["lore.hidden", "Hidden notes — author-only, never sent to AI"], ["lore.requests", "In AI requests"],
  ["lore.include", "Include"], ["lore.known", "Known, don't reveal"],
  ["lore.leaveOut", "Leave out"], ["lore.pin", "Pin in AI context"],
  ["lore.kind.character", "character"], ["lore.kind.place", "place"],
  ["lore.kind.term", "term"], ["lore.kind.note", "note"],
  ["lore.window.title", "Lore in window"], ["lore.window.context", "Continue context · includes pins · excludes leave out and budget skips."],
  ["lore.window.empty", "No lore in window."], ["lore.window.match", "Lore match"],
  ["lore.window.matches", "{countText} matches"], ["lore.window.noVisible", "No visible text."],
  ["lore.window.shortcut", "Alt+L opens · Esc closes"],
])("tree and lore message %s preserves exact English and has Spanish", (key, english) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key)).toBe(english);
});

test("tree and lore placeholders preserve assembled English", () => {
  expect(formatMessage("en", "tree.rename", { title: "Ada's {draft}" })).toBe("Rename 'Ada's {draft}'");
  expect(formatMessage("en", "tree.delete", { title: "Draft" })).toBe("Delete 'Draft'");
  expect(formatMessage("en", "tree.deleted", { title: "Draft" })).toBe("Deleted 'Draft'.");
  expect(formatMessage("en", "tree.returnFailed", { message: "offline" })).toBe("Item deleted, but returning to the library failed: offline Use Undo to restore it.");
  for (const count of [0, 1, 1000]) expect(formatMessage("en", "lore.window.matches", { count, countText: String(count) })).toBe(`${count} matches`);
  expect(formatMessage("es", "lore.window.matches", { count: 1, countText: "1" })).toBe("1 coincidencia");
  expect(formatMessage("es", "lore.window.matches", { count: 1000, countText: "1000" })).toBe("1000 coincidencias");
});

test("timeline move labels interpolate the point name in both locales", () => {
  expect(formatMessage("en", "timeline.moveUp", { name: "Act one" })).toBe("Move Act one up");
  expect(formatMessage("es", "timeline.moveDown", { name: "Acto uno" })).toBe("Bajar Acto uno");
  expect(formatMessage("es", "timeline.base")).toBe("Sin punto (texto base)");
  expect(formatMessage("en", "timeline.inherit", { name: "Act one" })).toBe("Inherit: Act one");
  expect(formatMessage("es", "timeline.inherit", { name: "Acto uno" })).toBe("Heredar: Acto uno");
});

test.each(["Chapter", "Scene", "Universe", "Container", "Piece", "unknown", "toString"])("creation messages select full labels for %s", noun => {
  const known = ["Chapter", "Scene", "Universe", "Container", "Piece"].includes(noun);
  const name = known ? noun : "Item";
  const keys = messages.treeCreateMessageKeys(noun);
  expect(formatMessage("en", keys.button)).toBe(name);
  expect(formatMessage("en", keys.title)).toBe(`${name} title`);
  expect(formatMessage("en", keys.creating)).toBe(`Creating ${name.toLowerCase()}…`);
  expect(formatMessage("es", keys.title)).toBe(name === "Scene" ? "Título de la escena" : name === "Chapter" ? "Título del capítulo" : name === "Universe" ? "Título del universo" : name === "Container" ? "Título del contenedor" : name === "Piece" ? "Título del texto" : "Título del elemento");
});

test.each([
  ["editor.windowSize", "Window Size"],
  ["editor.wholeDocument", "Whole document"],
  ["editor.navigation", "Navigation"],
  ["editor.backToLibrary", "Back to library"],
  ["editor.manuscript", "Manuscript"],
  ["editor.aiTools", "AI tools"],
  ["editor.stop", "Stop"],
  ["editor.continue", "Continue"],
  ["editor.editInstruction", "Edit instruction"],
  ["editor.editPlaceholder", "How should the selected text change?"],
  ["editor.editHint", "Enter applies · Esc or Edit closes · Shift+Enter adds a line"],
  ["editor.apply", "Apply"],
  ["editor.contextInspector", "Context inspector"],
  ["editor.settings", "Settings"],
  ["editor.showContinueWindow", "Show Continue window"],
  ["editor.tintAiText", "Tint AI text"],
  ["editor.manuscriptOutline", "Manuscript outline"],
  ["editor.deleteWait", "Wait for the current action before deleting."],
  ["editor.deleteNotReady", "Editor is not ready to delete this item."],
  ["editor.deleteSaveFailed", "Could not save the open document before deleting."],
  ["editor.deleteChanged", "Editor changed before deleting this item."],
  ["editor.openForLore", "Open a piece or chapter to manage its universe lore."],
  ["editor.download", "Download {format}"],
  ["editor.textFormat", "text"],
  ["editor.exportChapterFailed", "Could not export chapter. Try again."],
  ["editor.book", "Book:"],
  ["editor.exportBookFailed", "Could not export book. Try again."],
  ["editor.downloadBackup", "Download backup"],
  ["editor.recoveryNotice", "Previous unsaved work is available. Export it before closing this tab."],
  ["editor.exportRecovery", "Export recovery"],
  ["editor.chapterSaveStatus", "Chapter save status"],
  ["editor.newScene", "New scene"],
  ["editor.blockType", "Block type"],
  ["editor.createFirstChapter", "Create the first chapter"],
  ["editor.documentCounts", "Document counts"],
  ["shortcut.stop", "Stop AI"],
  ["shortcut.continue", "Continue writing"],
  ["shortcut.edit", "Edit selection"],
  ["landing.library", "Library"],
  ["tree.noChapters", "No chapters yet."],
])("editor surface message %s preserves exact English and has Spanish", (key, english) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key)).toBe(english);
});

test("editor download preserves assembled English", () => {
  expect(formatMessage("en", "editor.download", { format: "Markdown" })).toBe("Download Markdown");
  expect(formatMessage("en", "editor.download", { format: "text" })).toBe("Download text");
});

test.each([
  ["inspector.liveContext", "Live Prompt Context"],
  ["inspector.storyContext", "Story Context"],
  ["inspector.storyPlaceholder", "Series premise, tone, point of view, or scene notes"],
  ["inspector.continueInstruction", "Continue instruction"],
  ["inspector.continuePlaceholder", "Continue the scene, build suspense, introduce a new obstacle..."],
  ["inspector.editNote", "Edit replaces the current selection as text arrives. Stopping an Edit keeps text that already landed; failed edits restore the original passage. Desktop Continue Stop removes only AI text and keeps your typing."],
  ["inspector.advanced", "Advanced AI controls"],
  ["inspector.maxOutput", "Max Output Tokens"],
  ["inspector.resumePrompt", "Thread continuation prompt"],
  ["inspector.guidance", "Author guidance"],
  ["inspector.saveGuidance", "Save author guidance"],
  ["inspector.guidanceNote", "Saved on the server for all requests. Empty text removes guidance. Limit: 65,536 UTF-8 bytes. System rules are read-only."],
  ["inspector.request", "Request to inspect"],
  ["inspector.overrideNote", "Edit content for the next selected request. Tagged content is escaped; section tags and selection ranges stay fixed. History keeps separate speaker turns. Overrides reset after accepted sends or chapter changes. Limits: 65,536 UTF-8 bytes per section, 262,144 total."],
  ["inspector.modelCost", "Model and cost"],
  ["inspector.pendingEstimate", "Pending estimate"],
  ["settings.ai.model", "Model"],
  ["inspector.inputCost", "Estimated input cost"],
  ["inspector.outputRate", "Output rate"],
  ["inspector.actualRun", "Most recent run · actual"],
  ["inspector.requestModel", "Request model"],
  ["inspector.observedUsage", "Observed usage"],
  ["inspector.actualCost", "Actual cost"],
  ["inspector.costNote", "Observed usage only, not a final bill. No cache discount assumed."],
  ["inspector.estimatedTokens", "{tokens} estimated tokens"],
  ["inspector.injectedLore", "Injected lore"],
  ["inspector.loreBudget", "Next request · {characters} UTF-16 character budget. Pins can exceed it."],
  ["inspector.loreTokenNote", "Tokens: UTF-8 bytes ÷ 4, rounded up per entry including standalone Lore tags; not additive."],
  ["inspector.noLore", "No lore injected."],
  ["inspector.pin", "pin"],
  ["inspector.stateBase", "base"],
  ["inspector.stateNamed", "state {name}"],
  ["inspector.keyword", "keyword"],
  ["inspector.skipped", "Skipped by budget"],
  ["inspector.noSkipped", "No entries skipped by budget."],
  ["inspector.sectionLabel", "{section} prompt section"],
  ["inspector.historyTurn", "{role} turn {number}"],
  ["inspector.noHistory", "No history turns."],
  ["inspector.addTurn", "Add {role} turn"],
  ["inspector.resetSection", "Reset {section}"],
  ["inspector.copied", "Copied"],
  ["inspector.totalTokens", "Total: {tokens} estimated tokens"],
  ["inspector.fullPrompt", "Full prompt"],
  ["inspector.copy", "Copy"],
  ["inspector.copyFailed", "Could not copy prompt."],
  ["editor.continue", "Continue"],
  ["shortcut.edit", "Edit selection"],
  ["lore.known", "Known, don't reveal"],
  ["lore.include", "Include"],
  ["thread.resumeOption", "Continue · from a cut-off reply"],
])("inspector message %s preserves exact English and has Spanish", (key, english) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key)).toBe(english);
});

test("inspector placeholders preserve assembled English and raw data", () => {
  expect(formatMessage("en", "inspector.sectionLabel", { section: "Author {guidance}" })).toBe("Author {guidance} prompt section");
  expect(formatMessage("en", "inspector.historyTurn", { role: "assistant", number: "1000" })).toBe("assistant turn 1000");
  expect(formatMessage("en", "inspector.totalTokens", { tokens: "1000" })).toBe("Total: 1000 estimated tokens");
  expect(formatMessage("en", "inspector.loreBudget", { characters: "1,000" })).toBe("Next request · 1,000 UTF-16 character budget. Pins can exceed it.");
  expect(formatMessage("en", "inspector.estimatedTokens", { tokens: "1000" })).toBe("1000 estimated tokens");
  expect(formatMessage("en", "inspector.addTurn", { role: "user" })).toBe("Add user turn");
  expect(formatMessage("en", "inspector.resetSection", { section: "Story Context" })).toBe("Reset Story Context");
});

test.each([
  ["editor.continueWindow", "Continue window"],
  ["editor.windowStarts", "starts ↑ {position}"],
  ["editor.justAbove", "just above"],
  ["editor.pagesUp", "{pages} pages up"],
  ["lore.entryLabel", "{name} · {kind}{pinned}"],
  ["lore.pinned", " · pinned"],
  ["lore.known", "Known, don't reveal"],
  ["lore.leaveOut", "Leave out"],
  ["lore.include", "Include"],
  ["lore.pinHint", "Sent with every AI request unless set to Leave out, even when the text doesn't mention it, and never dropped for space."],
  ["lore.termsHint", "Whole words or phrases, any case. When the name, an alias or a keyword appears in the text, this entry is sent to the AI."],
  ["textCounts.word", "{count} word"],
  ["draft.words", "{count} words"],
  ["textCounts.character", "{count} character"],
  ["textCounts.characters", "{count} characters"],
  ["textCounts.line", "{count} line"],
  ["textCounts.lines", "{count} lines"],
  ["textCounts.selected", "{count} of "],
  ["textCounts.separator", " · "],
  ["editor.emptyContinueBudget", "The model used its whole budget without writing any text. Try again or pick another model."],
  ["editor.emptyContinue", "The model returned no text."],
  ["editor.notSaved", "Not saved"],
  ["editor.continuing", "Continuing"],
  ["editor.editing", "Editing"],
  ["editor.cancelling", "Cancelling"],
  ["editor.cancelled", "Cancelled"],
  ["editor.done", "Done"],
  ["editor.idle", "Idle"],
  ["script.scenesChanged", "Scenes changed; reload"],
])("editor helper message %s preserves exact English and has Spanish", (key, english) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key)).toBe(english);
});

test("editor helper placeholders preserve assembled English and raw data", () => {
  const above = formatMessage("en", "editor.justAbove");
  expect(formatMessage("en", "editor.windowStarts", { position: above })).toBe("starts ↑ just above");
  for (const pages of [1, 2, 1000]) {
    const position = formatMessage("en", "editor.pagesUp", { pages: String(pages) });
    expect(position).toBe(`${pages} pages up`);
    expect(formatMessage("en", "editor.windowStarts", { position })).toBe(`starts ↑ ${pages} pages up`);
  }
  expect(formatMessage("en", "lore.entryLabel", { name: "Mara {name}", kind: "character", pinned: "" })).toBe("Mara {name} · character");
  expect(formatMessage("en", "lore.entryLabel", { name: "Mara", kind: "character", pinned: formatMessage("en", "lore.pinned") })).toBe("Mara · character · pinned");
  for (const [key, label] of [["textCounts.word", "1 word"], ["draft.words", "1 words"], ["textCounts.character", "1 character"], ["textCounts.characters", "1 characters"], ["textCounts.line", "1 line"], ["textCounts.lines", "1 lines"]]) {
    expect(formatMessage("en", key!, { count: "1" })).toBe(label);
  }
  expect(formatMessage("en", "textCounts.selected", { count: "1,000" })).toBe("1,000 of ");
});

test.each([
  [0, "0 words · 0 characters · 0 lines"],
  [1, "1 word · 1 character · 1 line"],
  [2, "2 words · 2 characters · 2 lines"],
  [1000, "1,000 words · 1,000 characters · 1,000 lines"],
] as const)("editor count labels preserve assembled English for %s", (count, english) => {
  expect(formatTextCounts({ words: count, characters: count, lines: count })).toBe(english);
  expect(formatTextCounts({ words: count, characters: count, lines: count }, 1)).toBe(`1 of ${english}`);
});

test.each([
  ["sync.onlyTextPiece", "Only a piece with text can open in the editor.", {}],
  ["sync.defaultSeries", "My Series", {}],
  ["sync.defaultBook", "My Book", {}],
  ["sync.firstChapter", "Chapter One", {}],
  ["sync.pieceUrlInvalid", "Piece URL requires a positive pieceId.", {}],
  ["sync.chapterUrlInvalid", "Chapter URL requires positive seriesId, bookId, and chapterId.", {}],
  ["sync.noChapter", "No chapter found. Create a chapter to start editing.", {}],
  ["sync.unknownSynchronization", "Unknown synchronization error", {}],
  ["sync.saveStopped", "Save stopped: server {message}: unavailable. Export local work before reload. Do not retry an uncertain submission.", {"message":"server {message}: unavailable."}],
  ["sync.connectionInterrupted", "Connection interrupted: server {message}: unavailable. Local work retained; polling will retry.", {"message":"server {message}: unavailable."}],
  ["sync.saveRetry", "Save stopped: server {message}: unavailable. Local work retained; polling will retry.", {"message":"server {message}: unavailable."}],
  ["sync.sessionStopped", "Chapter session stopped.", {}],
  ["sync.aiReserved", "Cannot save chapter while AI has reserved the session.", {}],
  ["sync.missingRevision", "Missing chapter revision; local work retained.", {}],
  ["sync.acknowledgementMismatch", "Chapter acknowledgement mismatch; local work retained.", {}],
  ["sync.acknowledgementMissing", "Chapter acknowledgement not in history; local work retained.", {}],
  ["sync.loadingChapter", "Loading chapter…", {}],
  ["sync.unsavedChanges", "Unsaved changes", {}],
  ["sync.saved", "Saved", {}],
  ["sync.recoveryExport", " · Recovery storage unavailable; export before closing", {}],
  ["sync.recoveryUnavailable", " · Recovery storage unavailable", {}],
  ["sync.status", "My {title} · Saved · Recovery storage unavailable", {"title":"My {title}","status":"Saved","recovery":" · Recovery storage unavailable"}],
  ["sync.chapterSwitchFailed", "Chapter switch failed: server {message}: unavailable. Current chapter retained.", {"message":"server {message}: unavailable."}],
  ["sync.unknownError", "Unknown error", {}],
  ["sync.chapterLoadFailed", "Chapter load failed.", {}],
  ["sync.savingBeforeLeaving", "Saving before leaving…", {}],
  ["sync.leavingChapterStopped", "Leaving stopped: server {message}: unavailable. Current chapter retained.", {"message":"server {message}: unavailable."}],
  ["sync.chapterCreationFailed", "Chapter creation failed.", {}],
  ["sync.chapterSessionChanged", "Chapter session changed before starting AI.", {}],
  ["script.breakNotice", "Edits across a scene break are not saved", {}],
  ["script.editAcrossBreak", "Select text inside one scene to use Edit.", {}],
  ["script.loading", "Loading script…", {}],
  ["script.syncByPiece", "Script scenes sync by piece.", {}],
  ["script.sceneProblem", "Scene {title}: server {message}: unavailable.", {"title":"Scene {title}","message":"server {message}: unavailable."}],
  ["script.loadFailed", "Script load failed.", {}],
  ["script.leavingStopped", "Leaving stopped: server {message}: unavailable. Current script retained.", {"message":"server {message}: unavailable."}],
  ["script.sceneTitle", "Scene 1000", {"number":"1000"}],
  ["script.sceneNotAdded", "New scene not added: server {message}: unavailable.", {"message":"server {message}: unavailable."}],
  ["script.savingNewScene", "Saving script before opening the new scene…", {}],
  ["script.savingBeforeOpening", "Saving script before opening…", {}],
  ["script.sessionChanged", "Script session changed before starting AI.", {}],
  ["tree.create.chapter.creating", "Creating chapter…", {}],
] as const)("sync message %s preserves exact English and raw inserted text", (key, english, values) => {
  expect(Object.hasOwn(en, key)).toBe(true);
  expect(Object.hasOwn(es, key)).toBe(true);
  expect(formatMessage("en", key, values)).toBe(english);
});

test("explicit sync failure state overrides status wording", () => {
  expect(getSavePresentation("Guardado", "Texto", false, true)).toEqual({ label: "Not saved", failed: true, detail: "Guardado" });
  expect(getSavePresentation("Saved · export", null, false, false)).toEqual({ label: "Saved · export", failed: false });
  expect(getSavePresentation("Guardado", null, true, false)).toEqual({ label: "Not saved", failed: true, detail: "Guardado" });
});
