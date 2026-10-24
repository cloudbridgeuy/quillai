import { expect, test } from "bun:test";
import { createTitle, draftTransition, nextPosition, targetPosition, type ManuscriptSeries } from "./create_draft";

test("titles reject blank input and trim accepted input", () => {
  for (const value of ["", " \t\n", "\u00a0"]) expect(createTitle(value)).toBeNull();
  expect(createTitle("  Chapter 😀  ")).toBe("Chapter 😀");
});

test("positions append after the maximum, not the count or final entry", () => {
  expect(nextPosition([])).toBe(0);
  expect(nextPosition([{ position: 8 }, { position: 2 }])).toBe(9);
  const series: ManuscriptSeries[] = [{ id: 1, title: "Series", position: 5, books: [
    { id: 2, series_id: 1, title: "Book", position: 3, chapters: [
      { id: 3, title: "Chapter", position: 7, kind: "chapter", head_rev: 0 },
    ] },
  ] }];
  expect(targetPosition(series, { kind: "series" })).toBe(6);
  expect(targetPosition(series, { kind: "book", seriesId: 1 })).toBe(4);
  expect(targetPosition(series, { kind: "chapter", seriesId: 1, bookId: 2 })).toBe(8);
  expect(() => targetPosition(series, { kind: "book", seriesId: 9 })).toThrow();
  expect(() => targetPosition(series, { kind: "chapter", seriesId: 1, bookId: 9 })).toThrow();
});

test("draft rejects blanks, retains failures, locks requests and cancels", () => {
  const started = draftTransition(null, { type: "start", target: { kind: "series" } });
  expect(draftTransition(started, { type: "submit" })).toMatchObject({ pending: false, error: "Enter a title." });
  const titled = draftTransition(started, { type: "title", title: "New" });
  const pending = draftTransition(titled, { type: "submit" });
  expect(pending?.pending).toBe(true);
  for (const action of [{ type: "cancel" }, { type: "submit" }, { type: "title", title: "Other" },
    { type: "start", target: { kind: "book", seriesId: 1 } }] as const) expect(draftTransition(pending, action)).toBe(pending);
  const failed = draftTransition(pending, { type: "failed", message: "Offline" });
  expect(failed).toMatchObject({ title: "New", pending: false, error: "Offline" });
  expect(draftTransition(failed, { type: "title", title: "Retry" })?.error).toBeNull();
  expect(draftTransition(failed, { type: "cancel" })).toBeNull();
  expect(draftTransition(pending, { type: "created" })).toBeNull();
  expect(draftTransition(null, { type: "submit" })).toBeNull();
});
