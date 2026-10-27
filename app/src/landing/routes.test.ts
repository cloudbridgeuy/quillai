import { describe, expect, test } from "bun:test";
import { editorUrl, landingUrl, parseLastEdited, scriptGroupId, showsLanding } from "./routes";

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

  test("opens script groups and their scenes as one surface", () => {
    expect(scriptGroupId({ id: 4, format: "script", shape: "group", parentId: null })).toBe(4);
    expect(scriptGroupId({ id: 5, format: "script", shape: "leaf", parentId: 4 })).toBe(4);
    expect(scriptGroupId({ id: 6, format: "script", shape: "leaf", parentId: null })).toBeUndefined();
    expect(scriptGroupId({ id: 7, format: "prose", shape: "leaf", parentId: 3 })).toBeUndefined();
  });
});
