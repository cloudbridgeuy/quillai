// Generates language-neutral Delta fixtures into <repo>/fixtures/delta using quill-delta.
// Run: bun run fixtures:delta
import Delta from "quill-delta";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

type Ops = Delta["ops"];
const d = (ops: Ops) => new Delta(ops);
const wire = (x: Delta) => ({ ops: x.ops });

const IMG = { image: "https://example.com/a.png" };
const IMG2 = { image: "https://example.com/b.png" };
const VID = { video: "https://example.com/v.mp4" };

// Named pool of change-style deltas (may contain retain/delete).
const pool: Record<string, Ops> = {
  empty: [],
  text: [{ insert: "Hello" }],
  textBold: [{ insert: "Hello", attributes: { bold: true } }],
  mixed: [{ insert: "Hello " }, { insert: "World", attributes: { bold: true } }, { insert: "\n" }],
  retain3: [{ retain: 3 }],
  retainBold: [{ retain: 3, attributes: { bold: true } }],
  retainUnbold: [{ retain: 3, attributes: { bold: null } }],
  retainMulti: [{ retain: 2 }, { retain: 2, attributes: { italic: true, color: "red" } }],
  delete2: [{ delete: 2 }],
  deleteStart: [{ delete: 1 }, { insert: "X" }],
  retainDelete: [{ retain: 2 }, { delete: 3 }],
  insMid: [{ retain: 2 }, { insert: "ab" }],
  insEmbed: [{ retain: 1 }, { insert: IMG }],
  embedOnly: [{ insert: IMG }],
  embedAttr: [{ insert: IMG2, attributes: { alt: "b", width: "10" } }],
  video: [{ insert: VID }],
  emoji: [{ insert: "a😀b" }],
  emojiRetain: [{ retain: 2 }, { insert: "👨‍👩‍👧" }],
  cjk: [{ insert: "日本語" }, { delete: 1 }],
  combining: [{ insert: "éx" }],
  lineAttr: [{ insert: "Title" }, { insert: "\n", attributes: { header: 1 } }],
  retainLine: [{ retain: 5 }, { retain: 1, attributes: { header: 2 } }],
  newlines: [{ insert: "a\n\nb\n" }],
  deleteLong: [{ delete: 10 }],
  attrNested: [{ insert: "link", attributes: { link: "https://x.y", bold: true } }],
  retainEmbedAttr: [{ retain: 1, attributes: { width: "20" } }, { insert: "z" }],
  zeroAttr: [{ retain: 4, attributes: {} }],
};
const names = Object.keys(pool);

type Case = { name: string; input: unknown; expected: unknown };
const out: Record<string, Case[]> = {
  compose: [], transform: [], transform_position: [], invert: [], diff: [],
};

// compose: all ordered pairs
for (const a of names) for (const b of names) {
  const A = d(pool[a]!), B = d(pool[b]!);
  out.compose!.push({
    name: `compose ${a} then ${b}`,
    input: { a: wire(A), b: wire(B) },
    expected: wire(A.compose(B)),
  });
}

// transform: every pair, both priorities (subset of pairs to keep size sane)
for (let i = 0; i < names.length; i++) for (let j = 0; j < names.length; j += 1) {
  if ((i + j) % 3 !== 0) continue;
  const a = names[i]!, b = names[j]!;
  const A = d(pool[a]!), B = d(pool[b]!);
  for (const priority of [true, false]) {
    out.transform!.push({
      name: `transform ${a} vs ${b} priority=${priority}`,
      input: { a: wire(A), b: wire(B), priority },
      expected: wire(A.transform(B, priority)),
    });
  }
}

// transformPosition
const posDeltas = names.filter((n) => !["empty"].includes(n));
posDeltas.push("empty");
for (const n of posDeltas) {
  const A = d(pool[n]!);
  for (const index of [0, 1, 2, 3, 5, 8]) for (const priority of [true, false]) {
    if ((index + n.length + (priority ? 1 : 0)) % 3 !== 0 && index !== 0 && index !== 3) continue;
    out.transform_position!.push({
      name: `transformPosition ${n} index=${index} priority=${priority}`,
      input: { delta: wire(A), index, priority },
      expected: A.transformPosition(index, priority),
    });
  }
}

// invert: base documents x applicable changes
const bases: Record<string, Ops> = {
  empty: [],
  plain: [{ insert: "Hello World\n" }],
  styled: [{ insert: "Hel", attributes: { bold: true } }, { insert: "lo " }, { insert: "World", attributes: { italic: true, color: "red" } }, { insert: "\n" }],
  embeds: [{ insert: "a" }, { insert: IMG }, { insert: "b", attributes: { bold: true } }, { insert: VID }, { insert: "\n" }],
  unicode: [{ insert: "a😀b日本語\n" }],
  headers: [{ insert: "Title" }, { insert: "\n", attributes: { header: 1 } }, { insert: "body\n" }],
};
for (const [bn, bo] of Object.entries(bases)) {
  const base = d(bo);
  for (const cn of names) {
    const change = d(pool[cn]!);
    // only changes whose retains/deletes fit inside base
    const consumed = change.ops.reduce((n, o) => n + (o.retain ? (o.retain as number) : o.delete ? (o.delete as number) : 0), 0);
    if (consumed > base.length()) continue;
    out.invert!.push({
      name: `invert ${cn} on ${bn}`,
      input: { change: wire(change), base: wire(base) },
      expected: wire(change.invert(base)),
    });
  }
}

// diff: document pairs
const docs: Record<string, Ops> = {
  empty: [],
  hello: [{ insert: "Hello" }],
  helloBold: [{ insert: "Hello", attributes: { bold: true } }],
  helloWorld: [{ insert: "Hello World\n" }],
  helloMixed: [{ insert: "Hello " }, { insert: "World", attributes: { bold: true } }, { insert: "\n" }],
  helloEmbed: [{ insert: "Hello" }, { insert: IMG }],
  embedA: [{ insert: IMG }],
  embedB: [{ insert: IMG2 }],
  emoji: [{ insert: "a😀b" }],
  emoji2: [{ insert: "a😁b" }],
  cjk: [{ insert: "日本語" }],
  cjk2: [{ insert: "日本人" }],
  header: [{ insert: "Hello" }, { insert: "\n", attributes: { header: 1 } }],
  lines: [{ insert: "a\nb\nc\n" }],
  linesEdit: [{ insert: "a\nB\nc\nd\n" }],
};
const dn = Object.keys(docs);
for (const a of dn) for (const b of dn) {
  const A = d(docs[a]!), B = d(docs[b]!);
  out.diff!.push({
    name: `diff ${a} to ${b}`,
    input: { a: wire(A), b: wire(B) },
    expected: wire(A.diff(B)),
  });
}
// diff with cursor hint
{
  const A = d([{ insert: "abcabc" }]), B = d([{ insert: "abcabcabc" }]);
  for (const cursor of [0, 3, 6]) {
    out.diff!.push({
      name: `diff abcabc to abcabcabc cursor=${cursor}`,
      input: { a: wire(A), b: wire(B), cursor },
      expected: wire(A.diff(B, cursor)),
    });
  }
}

const dir = join(import.meta.dir, "../../fixtures/delta");
mkdirSync(dir, { recursive: true });
let total = 0;
for (const [op, cases] of Object.entries(out)) {
  writeFileSync(join(dir, `${op}.json`), JSON.stringify(cases, null, 2) + "\n");
  console.log(`${op}: ${cases.length}`);
  total += cases.length;
}
console.log(`total: ${total}`);
