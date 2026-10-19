// Markdown analysis types (Type-Driven Development)

export type MarkdownFormat = { bold?: true; italic?: true };

export type MarkdownPattern = {
  regex: RegExp;
  format: MarkdownFormat;
  type: 'bold-italic' | 'bold-italic-underscore' | 'bold' | 'italic' | 'italic-underscore';
  markersLength: number;
};

export type MarkdownMatch = {
  match: RegExpMatchArray;
  pattern: MarkdownPattern;
  start: number;
  end: number;
  content: string;
};

// Pattern analysis with precedence (longest first to avoid premature matching)
const patterns: MarkdownPattern[] = [
  {
    regex: /\*\*\*([^*]+)\*\*\*$/,
    format: { bold: true, italic: true },
    type: 'bold-italic',
    markersLength: 3
  },
  // Mixed bold with underscore italic (must come before plain bold)
  {
    regex: /\*\*_([^_*]+)_\*\*$/,
    format: { bold: true, italic: true },
    type: 'bold-italic-underscore',
    markersLength: 5
  },
  {
    regex: /\*\*([^*]+)\*\*$/,
    format: { bold: true },
    type: 'bold',
    markersLength: 2
  },
  {
    regex: /\*([^*]+)\*$/,
    format: { italic: true },
    type: 'italic',
    markersLength: 1
  },
  // Underscore italic
  {
    regex: /_([^_]+)_$/,
    format: { italic: true },
    type: 'italic-underscore',
    markersLength: 1
  }
];

// Pure function: string in, typed match out (Parse Don't Validate)
export function analyzeMarkdownContext(text: string, cursorPos: number): MarkdownMatch | null {
  const beforeCursor = text.substring(0, cursorPos);

  for (const pattern of patterns) {
    const match = beforeCursor.match(pattern.regex);
    if (match) {
      const matchStart = match.index ?? 0;
      const matchEnd = matchStart + match[0].length;

      return {
        match,
        pattern,
        start: matchStart,
        end: matchEnd,
        content: match[1]
      };
    }
  }

  return null;
}

// Pure function: checks if a markdown pattern is complete and ready for processing
export function isPatternComplete(text: string, cursorPos: number): boolean {
  const context = analyzeMarkdownContext(text, cursorPos);
  if (!context) return false;

  const afterCursor = text.substring(cursorPos);
  const nextChar = afterCursor.charAt(0);

  // Next char is asterisk — user may be typing a longer pattern
  if (nextChar === '*') {
    return false;
  }

  // Italic might become bold or bold-italic
  if (context.pattern.type === 'italic') {
    const beforePattern = text.substring(Math.max(0, context.start - 3), context.start);
    if (beforePattern.endsWith('*') || beforePattern.endsWith('**')) {
      return false;
    }
  }

  // Bold might become bold-italic
  if (context.pattern.type === 'bold') {
    const beforePattern = text.substring(Math.max(0, context.start - 1), context.start);
    if (beforePattern.endsWith('*')) {
      return false;
    }
  }

  // Underscore italic might become a mixed pattern
  if (context.pattern.type === 'italic-underscore') {
    const beforePattern = text.substring(Math.max(0, context.start - 2), context.start);
    if (beforePattern.endsWith('*') || beforePattern.endsWith('**')) {
      return false;
    }
  }

  return true;
}
