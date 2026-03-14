import { test, expect } from "bun:test";
import { analyzeMarkdownContext, isPatternComplete } from "./markdown";

// analyzeMarkdownContext tests

test("*foo* at cursor after closing * returns italic match", () => {
  const result = analyzeMarkdownContext("*foo*", 5);
  expect(result).not.toBeNull();
  expect(result!.pattern.type).toBe("italic");
  expect(result!.content).toBe("foo");
  expect(result!.start).toBe(0);
  expect(result!.end).toBe(5);
});

test("**foo** at cursor after closing ** returns bold match", () => {
  const result = analyzeMarkdownContext("**foo**", 7);
  expect(result).not.toBeNull();
  expect(result!.pattern.type).toBe("bold");
  expect(result!.content).toBe("foo");
});

test("***foo*** at cursor after closing *** returns bold-italic match", () => {
  const result = analyzeMarkdownContext("***foo***", 9);
  expect(result).not.toBeNull();
  expect(result!.pattern.type).toBe("bold-italic");
  expect(result!.content).toBe("foo");
  expect(result!.pattern.format).toEqual({ bold: true, italic: true });
});

test("*foo (incomplete) returns null", () => {
  const result = analyzeMarkdownContext("*foo", 4);
  expect(result).toBeNull();
});

test("**foo* (mismatched) returns italic match on foo*", () => {
  // The italic regex /\*([^*]+)\*$/ would match the trailing *foo* portion
  // but [^*]+ prevents matching because the text after ** starts at position 2
  // Actually **foo* - the italic regex looks for *X* where X has no asterisks
  // It would match *foo* starting at index 1 (the second *)
  const result = analyzeMarkdownContext("**foo*", 6);
  // The italic pattern *([^*]+)* would match starting at index 1: *foo*
  expect(result).not.toBeNull();
  expect(result!.pattern.type).toBe("italic");
  expect(result!.content).toBe("foo");
});

test("empty string returns null", () => {
  const result = analyzeMarkdownContext("", 0);
  expect(result).toBeNull();
});

test("no asterisks returns null", () => {
  const result = analyzeMarkdownContext("hello world", 11);
  expect(result).toBeNull();
});

test("*foo bar* returns italic match with content 'foo bar'", () => {
  const result = analyzeMarkdownContext("*foo bar*", 9);
  expect(result).not.toBeNull();
  expect(result!.pattern.type).toBe("italic");
  expect(result!.content).toBe("foo bar");
});

// isPatternComplete tests

test("*foo* is complete italic", () => {
  expect(isPatternComplete("*foo*", 5)).toBe(true);
});

test("**foo** is complete bold", () => {
  expect(isPatternComplete("**foo**", 7)).toBe(true);
});

test("***foo*** is complete bold-italic", () => {
  expect(isPatternComplete("***foo***", 9)).toBe(true);
});

test("*foo (incomplete) is not complete", () => {
  expect(isPatternComplete("*foo", 4)).toBe(false);
});

// isPatternComplete disambiguation tests

test("isPatternComplete returns false when next char is asterisk", () => {
  // "*foo*" followed by another "*" — might become "**foo**"
  const result = isPatternComplete("*foo**", 5);
  expect(result).toBe(false);
});

test("isPatternComplete returns false when italic might become bold", () => {
  // "**foo*" — asterisk before pattern start suggests longer pattern
  const result = isPatternComplete("**foo*", 6);
  expect(result).toBe(false);
});

test("isPatternComplete returns false when bold might become bold-italic", () => {
  // "***foo**" — asterisk before pattern start suggests longer pattern
  const result = isPatternComplete("***foo**", 8);
  expect(result).toBe(false);
});

// analyzeMarkdownContext cursor mid-text tests

test("analyzeMarkdownContext with cursor mid-text returns match", () => {
  const result = analyzeMarkdownContext("*foo* bar", 5);
  expect(result).not.toBeNull();
  expect(result!.content).toBe("foo");
  expect(result!.pattern.type).toBe("italic");
});
