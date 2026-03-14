# Markdown Parsing — FC/IS Extraction

## Architecture

The markdown syntax detection follows Functional Core / Imperative Shell:

**Functional Core** (`app/src/md_editor/markdown.ts`):
- `analyzeMarkdownContext(text, cursorPos)` — pure function, parses raw text + cursor position into typed `MarkdownMatch | null`
- `isPatternComplete(text, cursorPos)` — pure function, checks if a pattern is definitively complete (not mid-typing)
- Pattern precedence: bold-italic (`***`) > bold (`**`) > italic (`*`)

**Imperative Shell** (`app/src/md_editor/components/Editor.tsx`):
- `processMarkdownSyntax(quill, delta, source)` — reads Quill state, computes line boundaries and relative cursor position, calls pure functions, applies Quill delta mutations

## Types (Parse Don't Validate)

```ts
type MarkdownFormat = { bold?: true; italic?: true };
type MarkdownPattern = { regex: RegExp; format: MarkdownFormat; type: 'bold-italic' | 'bold' | 'italic'; markersLength: number };
type MarkdownMatch = { match: RegExpMatchArray; pattern: MarkdownPattern; start: number; end: number; content: string };
```

Raw text is parsed into a typed `MarkdownMatch` at the boundary — downstream code never works with raw strings.

## Key Detail: Relative Cursor Position

The shell computes `relativePos = currentPos - lineStart` before calling pure functions. The pure functions always receive cursor positions relative to the provided text, not absolute document positions.

## Tests

```sh
cd app && bun test src/md_editor/markdown.test.ts
```

16 tests covering: italic, bold, bold-italic, incomplete patterns, mismatched markers, empty input, spaces in content, disambiguation logic (next-char-asterisk, italic-might-become-bold, bold-might-become-bold-italic), and cursor mid-text.
