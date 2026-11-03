import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { countText, formatTextCounts } from "./text_counts";

describe("countText", () => {
  test("counts English and Spanish words and graphemes", () => {
    expect(countText("Hello mundo, ¡qué tal!\n")).toEqual({ words: 4, characters: 22, lines: 1 });
  });

  test("counts emoji and CJK using Unicode segmentation", () => {
    expect(countText("🪐\n")).toEqual({ words: 0, characters: 1, lines: 1 });
    expect(countText("你好世界\n")).toEqual({ words: 2, characters: 4, lines: 1 });
  });

  test("keeps blank lines and removes only the terminal newline", () => {
    expect(countText("\n")).toEqual({ words: 0, characters: 0, lines: 1 });
    expect(countText("a\n\n\n")).toEqual({ words: 1, characters: 1, lines: 3 });
  });

  test("excludes line breaks without joining words or graphemes across them", () => {
    expect(countText("one\ntwo\n")).toEqual({ words: 2, characters: 6, lines: 2 });
    expect(countText("e\u0301\n\u0301\n")).toEqual({ words: 1, characters: 2, lines: 2 });
    expect(countText("a\r\nb\u2028c\u2029\n").characters).toBe(3);
  });

  test("skips embeds in delta operations", () => {
    expect(countText([{ insert: "Hello " }, { insert: { image: "url" } }, { insert: "world\n" }])).toEqual({ words: 2, characters: 11, lines: 1 });
  });

  test("counts a range after an emoji and embed using Quill offsets", () => {
    const document = new Delta().insert("🪐 ").insert({ image: "url" }).insert(" hello world\n");
    expect(countText(document.slice(5, 10).ops, false).words).toBe(1);
  });

  test("counts a selection without removing its trailing newline", () => {
    expect(countText("one\n", false)).toEqual({ words: 1, characters: 3, lines: 2 });
  });
});

describe("formatTextCounts", () => {
  test("uses singular only for one and formats numbers in en-US", () => {
    expect(formatTextCounts({ words: 0, characters: 0, lines: 0 })).toBe("0 words · 0 characters · 0 lines");
    expect(formatTextCounts({ words: 1, characters: 1, lines: 1 })).toBe("1 word · 1 character · 1 line");
    expect(formatTextCounts({ words: 2, characters: 2, lines: 2 })).toBe("2 words · 2 characters · 2 lines");
    expect(formatTextCounts({ words: 1000, characters: 2000, lines: 1000 })).toBe("1,000 words · 2,000 characters · 1,000 lines");
  });

  test("uses the total word count for selection label plurality", () => {
    expect(formatTextCounts({ words: 2, characters: 6, lines: 1 }, 1)).toBe("1 of 2 words · 6 characters · 1 line");
    expect(formatTextCounts({ words: 1, characters: 1, lines: 1 }, 1)).toBe("1 of 1 word · 1 character · 1 line");
    expect(formatTextCounts({ words: 1, characters: 1, lines: 1 }, 0)).toBe("0 of 1 word · 1 character · 1 line");
  });
});
