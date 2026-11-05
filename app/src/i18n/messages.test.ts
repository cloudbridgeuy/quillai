import { expect, test } from "bun:test";
import * as messages from "./messages";
import { en, es, formatMessage, type MessageCatalog } from "./messages";

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

test.each(["Chapter", "Scene", "Universe", "Container", "Piece", "unknown", "toString"])("creation messages select full labels for %s", noun => {
  const known = ["Chapter", "Scene", "Universe", "Container", "Piece"].includes(noun);
  const name = known ? noun : "Item";
  const keys = messages.treeCreateMessageKeys(noun);
  expect(formatMessage("en", keys.button)).toBe(name);
  expect(formatMessage("en", keys.title)).toBe(`${name} title`);
  expect(formatMessage("en", keys.creating)).toBe(`Creating ${name.toLowerCase()}…`);
  expect(formatMessage("es", keys.title)).toBe(name === "Scene" ? "Título de la escena" : name === "Chapter" ? "Título del capítulo" : name === "Universe" ? "Título del universo" : name === "Container" ? "Título del contenedor" : name === "Piece" ? "Título del texto" : "Título del elemento");
});
