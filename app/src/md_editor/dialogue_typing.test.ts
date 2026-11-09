import { describe, expect, test } from "bun:test";
import { dialogueReplacement } from "./dialogue_typing";

const replace = (insert: string, remove = 1) => ({ remove, insert });

function typeAll(keys: string): string {
  let text = "";
  let ruleQuoteAt = -1;
  for (const key of keys) {
    const line = text.slice(text.lastIndexOf("\n") + 1);
    const result = key === "\n" ? null : dialogueReplacement(line, key, ruleQuoteAt === text.length - 1);
    text += key;
    ruleQuoteAt = -1;
    if (result) {
      text = text.slice(0, text.length - result.remove) + result.insert;
      if (result.insert === "‘") ruleQuoteAt = text.length - 1;
    }
  }
  return text;
}

describe("double quote", () => {
  test("opens at line start, after whitespace, and after opening punctuation", () => {
    for (const before of ["", "said ", "a\t", "(", "[", "{", "—", "–", "“", "‘", "«"]) {
      expect(dialogueReplacement(before, "\"")).toEqual(replace("“"));
    }
  });

  test("closes after a letter or punctuation", () => {
    for (const before of ["Hi", "Hi,", "end.", "no!", "x)"]) {
      expect(dialogueReplacement(before, "\"")).toEqual(replace("”"));
    }
  });

  test("opens after an em dash", () => {
    expect(typeAll("—\"Hola\"")).toBe("—“Hola”");
  });
});

describe("single quote", () => {
  test("opens at line start, after space, after ( and —", () => {
    for (const before of ["", "x ", "(", "—"]) {
      expect(dialogueReplacement(before, "'")).toEqual(replace("‘"));
    }
  });

  test("closes after a letter or punctuation", () => {
    expect(dialogueReplacement("don", "'")).toEqual(replace("’"));
    expect(dialogueReplacement("no.", "'")).toEqual(replace("’"));
  });

  test("don't and quoted words", () => {
    expect(typeAll("don't")).toBe("don’t");
    expect(typeAll("'Hi'")).toBe("‘Hi’");
  });

  test("nested quotes use the same context rule", () => {
    expect(typeAll("\"She said 'no'\"")).toBe("“She said ‘no’”");
  });

  test("rock 'n' roll gives an opening quote before n", () => {
    expect(typeAll("rock 'n' roll")).toBe("rock ‘n’ roll");
  });
});

describe("digit fix-up", () => {
  test("a rule-made opening quote before a digit becomes an apostrophe", () => {
    expect(dialogueReplacement("the ‘", "9", true)).toEqual(replace("’9", 2));
    expect(typeAll("the '90s")).toBe("the ’90s");
  });

  test("a hand-typed opening quote stays", () => {
    expect(dialogueReplacement("the ‘", "9", false)).toBeNull();
    expect(dialogueReplacement("the ‘", "9")).toBeNull();
    expect(typeAll("the ‘90s")).toBe("the ‘90s");
  });

  test("only the keystroke right after the rule-made quote fixes up", () => {
    expect(typeAll("'a9")).toBe("‘a9");
    expect(dialogueReplacement("the ‘", "x", true)).toBeNull();
  });
});

describe("dash", () => {
  test("-- becomes an em dash at line start and mid-line", () => {
    expect(dialogueReplacement("-", "-")).toEqual(replace("—", 2));
    expect(dialogueReplacement("Hola -", "-")).toEqual(replace("—", 2));
    expect(typeAll("--Hola --dijo--.")).toBe("—Hola —dijo—.");
  });

  test("a hyphen after an em dash stays, so --- is one dash", () => {
    expect(dialogueReplacement("—", "-")).toBeNull();
    expect(typeAll("---")).toBe("—-");
  });

  test("a single hyphen stays", () => {
    expect(dialogueReplacement("well", "-")).toBeNull();
    expect(typeAll("well-known")).toBe("well-known");
  });

  test("dash context is the same line only", () => {
    expect(typeAll("a-\n-b")).toBe("a-\n-b");
  });
});

describe("non-triggers", () => {
  test("other characters and multi-character inserts are ignored", () => {
    for (const typed of ["a", "*", "_", "“", "’", "—", " ", "\"\"", "--", "'x", ""]) {
      expect(dialogueReplacement("word ", typed)).toBeNull();
    }
  });

  test("digits without a preceding opening quote are ignored", () => {
    expect(dialogueReplacement("x", "9", true)).toBeNull();
  });
});
