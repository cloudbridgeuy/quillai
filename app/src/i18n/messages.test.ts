import { expect, test } from "bun:test";
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
