import type Delta from "quill-delta";

type Attributes = Record<string, unknown>;
type Line = { text: string; raw: string; attrs: Attributes };

function escapeText(text: string): string {
  return text.replace(/[\\`*_\[\]<]|~~/g, (marker) => marker === "~~" ? "\\~\\~" : `\\${marker}`);
}

function escapeLineStart(text: string): string {
  return text.replace(/^([ ]*)([#>+\-=|]|\d+[.)])/, (_, spaces: string, marker: string) => `${spaces}${marker.slice(0, -1)}\\${marker.slice(-1)}`);
}

function code(text: string): string {
  const fence = "`".repeat(Math.max(1, ...Array.from(text.matchAll(/`+/g), (match) => match[0].length + 1)));
  const padding = text.startsWith("`") || text.endsWith("`") || (text.startsWith(" ") && text.endsWith(" ") && text.trim() !== "") ? " " : "";
  return `${fence}${padding}${text}${padding}${fence}`;
}

function destination(value: string): string {
  return value.replace(/[\\()\s<>]/g, (character) => encodeURIComponent(character));
}

function safeUrl(value: unknown): value is string {
  return typeof value === "string" && /^https?:\/\//i.test(value);
}

function inline(text: string, attrs: Attributes): string {
  const match = /^(\s*)(.*?)(\s*)$/s.exec(text)!;
  const [, before, content, after] = match;
  if (!content && !attrs.code) return text;
  let result = attrs.code ? code(text) : escapeText(content!);
  if (attrs.bold && attrs.italic) result = `***${result}***`;
  else if (attrs.bold) result = `**${result}**`;
  else if (attrs.italic) result = `*${result}*`;
  if (attrs.strike) result = `~~${result}~~`;
  if (typeof attrs.link === "string") result = `[${result}](${destination(attrs.link)})`;
  return attrs.code ? result : `${before}${result}${after}`;
}

function lines(delta: Delta): Line[] {
  const result: Line[] = [];
  let text = "";
  let raw = "";
  let run = "";
  let runAttrs: Attributes = {};
  const flush = () => {
    if (!run) return;
    text += inline(run, runAttrs);
    run = "";
  };
  for (const op of delta.ops) {
    const attrs = op.attributes ?? {};
    if (typeof op.insert === "string") {
      const parts = op.insert.split("\n");
      parts.forEach((part, index) => {
        if (!["bold", "italic", "strike", "code", "link"].every((key) => attrs[key] === runAttrs[key])) flush();
        runAttrs = attrs;
        run += part;
        raw += part;
        if (index < parts.length - 1) {
          flush();
          result.push({ text, raw, attrs });
          text = "";
          raw = "";
        }
      });
    } else if (op.insert && typeof op.insert === "object") {
      flush();
      const embed = op.insert as Record<string, unknown>;
      if ("scene-break" in embed) {
        text += "***";
        raw += "***";
      } else if ("image" in embed) {
        const alt = typeof attrs.alt === "string" ? attrs.alt : "";
        text += safeUrl(embed.image) ? `![${escapeText(alt)}](${destination(embed.image)})` : escapeText(alt);
        raw += alt;
      } else if ("video" in embed && safeUrl(embed.video)) {
        text += `[video](${destination(embed.video)})`;
      } else if (typeof embed.formula === "string") {
        text += code(embed.formula);
        raw += embed.formula;
      }
    }
  }
  flush();
  if (text || raw) result.push({ text, raw, attrs: {} });
  return result;
}

function render(title: string, delta: Delta, markdown: boolean): string {
  const blocks: { kind: string; text: string; row?: unknown; language?: string }[] = [];
  for (const line of lines(delta)) {
    const { attrs } = line;
    let text = markdown ? line.text : line.raw;
    if (markdown && !attrs["code-block"]) text = escapeLineStart(attrs.table ? text.replace(/\|/g, "\\|") : text);
    if (attrs["scene-break"] !== undefined) text = "***";
    if (!text && !attrs.table && !attrs["code-block"]) continue;
    let kind = "paragraph";
    if (attrs.table) kind = "table";
    else if (attrs["code-block"]) {
      kind = "code";
      text = line.raw;
    } else if (markdown && attrs.list) {
      kind = "list";
      const marker = attrs.list === "ordered" ? "1. " : attrs.list === "checked" ? "- [x] " : attrs.list === "unchecked" ? "- [ ] " : "- ";
      const indent = typeof attrs.indent === "number" ? Math.max(0, Math.floor(attrs.indent)) : 0;
      text = " ".repeat(indent * (attrs.list === "ordered" ? 3 : 2)) + marker + text;
    } else if (markdown && typeof attrs.header === "number" && attrs.header >= 1 && attrs.header <= 6) text = `${"#".repeat(attrs.header)} ${text}`;
    else if (markdown && attrs.blockquote) text = `> ${text}`;
    const previous = blocks.at(-1);
    const language = typeof attrs["code-block"] === "string" && attrs["code-block"] !== "plain" ? attrs["code-block"].replace(/[\r\n`]/g, "") : "";
    if (kind === "table" && previous?.kind === "table") {
      previous.text += previous.row === attrs.table ? ` | ${text}` : `\n${text}`;
      previous.row = attrs.table;
    } else if ((kind === "list" || kind === "code") && previous?.kind === kind && (kind !== "code" || previous.language === language)) {
      previous.text += `\n${text}`;
    } else blocks.push({ kind, text, row: attrs.table, language });
  }
  const body = blocks.map((block) => {
    if (!markdown || block.kind !== "code") return block.text;
    const fence = "`".repeat(Math.max(3, ...Array.from(block.text.matchAll(/`+/g), (match) => match[0].length + 1)));
    return `${fence}${block.language}\n${block.text}\n${fence}`;
  }).join("\n\n");
  return `${markdown ? `# ${escapeText(title)}` : title}${body ? `\n\n${body}` : ""}\n`;
}

export function toMarkdown(title: string, delta: Delta): string {
  return render(title, delta, true);
}

export function toPlainText(title: string, delta: Delta): string {
  return render(title, delta, false);
}
