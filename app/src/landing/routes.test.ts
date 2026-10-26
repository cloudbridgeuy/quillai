import { describe, expect, test } from "bun:test";
import { editorUrl, landingUrl, parseLastEdited, showsLanding } from "./routes";

describe("landing routes", () => {
  test("shows landing only without an editor target", () => {
    expect(showsLanding("")).toBe(true);
    expect(showsLanding("?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A1")).toBe(true);
    expect(showsLanding("?pieceId=3")).toBe(false);
    expect(showsLanding("?seriesId=1&bookId=2&chapterId=3")).toBe(false);
    expect(showsLanding("?editor=1")).toBe(false);
  });

  test("keeps apiBaseUrl and replaces the editor target", () => {
    const href = "http://localhost:3001/?apiBaseUrl=x&editor=1&seriesId=1&bookId=2&chapterId=3";
    expect(landingUrl(href).search).toBe("?apiBaseUrl=x");
    expect(editorUrl(href, { universeId: 4, pieceId: 5 }).search).toBe("?apiBaseUrl=x&universeId=4&pieceId=5");
  });

  test("parses only a stored leaf scope with a title", () => {
    expect(parseLastEdited(JSON.stringify({ scope: { universeId: 1, pieceId: 2, extra: 1 }, title: "A" })))
      .toEqual({ scope: { universeId: 1, pieceId: 2 }, title: "A" });
    expect(parseLastEdited(JSON.stringify({ scope: { seriesId: 1, bookId: 2, chapterId: 3 }, title: "B" })))
      .toEqual({ scope: { seriesId: 1, bookId: 2, chapterId: 3 }, title: "B" });
    for (const raw of [null, "{", "[]", JSON.stringify({ scope: { pieceId: 2 }, title: "A" }),
      JSON.stringify({ scope: { universeId: 1, pieceId: 0 }, title: "A" }), JSON.stringify({ scope: { universeId: 1, pieceId: 2 } })]) {
      expect(parseLastEdited(raw)).toBeNull();
    }
  });
});
