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
  // Zero-length ops: quill-delta keeps them on parse and has quirks composing them.
  zeroInsert: [{ insert: "" }],
  zeroInsertAttr: [{ insert: "", attributes: { bold: true } }],
  zeroRetain: [{ retain: 0 }],
  zeroDelete: [{ delete: 0 }],
  zeroMix: [{ insert: "" }, { retain: 0 }, { insert: "a" }, { delete: 0 }, { retain: 1 }],
  zeroBetween: [{ retain: 1 }, { retain: 0 }, { insert: "" }, { delete: 2 }, { delete: 0 }],
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

// Odd-but-accepted ops (GUZ-718): quill-delta takes them at face value, so Rust must too.
// Cases where quill-delta throws (e.g. composing an embed retain with an embed) are skipped.
const odd: Record<string, Ops> = {
  // negative / fractional counts
  negRetain: [{ retain: -1 }],
  negRetainMid: [{ retain: 1 }, { retain: -2 }, { insert: "x" }],
  negDelete: [{ delete: -2 }],
  negDeleteMid: [{ retain: 1 }, { delete: -1 }],
  fracRetain: [{ retain: 1.5 }],
  fracRetainAttr: [{ retain: 2.5, attributes: { bold: true } }],
  fracDelete: [{ delete: 0.5 }],
  fracMix: [{ insert: "abc" }, { retain: 0.5 }, { delete: 1.5 }],
  // more than one of insert/retain/delete
  insDel: [{ insert: "a", delete: 1 } as any],
  insRet: [{ insert: "a", retain: 1 } as any],
  retDel: [{ retain: 2, delete: 1 } as any],
  allThree: [{ insert: "ab", retain: 1, delete: 2, attributes: { bold: true } } as any],
  retObjIns: [{ insert: "z", retain: { image: "x" } } as any],
  // unknown keys
  insUnknown: [{ insert: "a", foo: 1 } as any],
  insUnknownMid: [{ insert: "Hello", x: { y: [1, 2] } } as any, { retain: 2, note: "n" } as any],
  retUnknown: [{ retain: 2, foo: "x" } as any],
  delUnknown: [{ delete: 1, why: true } as any],
  embedUnknown: [{ insert: IMG, foo: 1 } as any],
  // attributes: null
  insAttrNull: [{ insert: "a", attributes: null } as any],
  retAttrNull: [{ retain: 2, attributes: null } as any],
  insAttrNullTwice: [{ insert: "a", attributes: null } as any, { insert: "b", attributes: null } as any],
  insAttrNullMixed: [{ insert: "a" }, { insert: "b", attributes: null } as any],
  embedAttrNull: [{ insert: IMG, attributes: null } as any],
  delAttrs: [{ delete: 1, attributes: { bold: true } } as any],
  delAttrNull: [{ delete: 2, attributes: null } as any],
  retNullThenText: [{ retain: 1, attributes: null } as any, { insert: "x", attributes: null } as any],
  // embed retains
  embedRet: [{ retain: { image: "x" } } as any],
  embedRetAttr: [{ retain: IMG2, attributes: { alt: "q" } } as any],
  embedRetMid: [{ retain: 1 }, { retain: { video: "v" } } as any, { insert: "t" }],
  embedRetDel: [{ retain: { image: "x" } } as any, { delete: 1 }],
  embedRetNull: [{ retain: { image: "x" }, attributes: null } as any],
  // any object is an op: no insert/retain/delete, or values of other types (length 1)
  emptyOp: [{} as any],
  emptyOpMid: [{ retain: 1 }, {} as any, { insert: "q" }],
  insNum: [{ insert: 5 } as any],
  insNull: [{ insert: null } as any],
  insBool: [{ retain: 1 }, { insert: true } as any],
  retStr: [{ retain: "3" } as any],
  retNull: [{ retain: null, attributes: { bold: true } } as any],
  delNull: [{ delete: null } as any],
  delStr: [{ retain: 1 }, { delete: "2" } as any],
  // non-object attributes
  insAttrNum: [{ insert: "a", attributes: 5 } as any],
  insAttrStr: [{ insert: "ab", attributes: "x" } as any, { insert: "c", attributes: "x" } as any],
  insAttrFalse: [{ insert: "a", attributes: false } as any],
  retAttrNum: [{ retain: 2, attributes: 7 } as any],
  retAttrZero: [{ retain: 2, attributes: 0 } as any],
  retAttrStr: [{ retain: 2, attributes: "xy" } as any],
  // array attributes: AttributeMap reads them as objects keyed by index
  insAttrArr: [{ insert: "ab", attributes: ["x", "y"] } as any],
  insAttrArrTwice: [{ insert: "a", attributes: ["b"] } as any, { insert: "c", attributes: ["b"] } as any],
  retAttrArr: [{ retain: 2, attributes: ["q", null] } as any],
  retAttrArrLong: [{ retain: 1, attributes: ["p", "q", "r"] } as any, { insert: "s" }],
  retAttrArrEmpty: [{ retain: 3, attributes: [] } as any],
  embedRetAttrArr: [{ retain: { image: "x" }, attributes: ["u"] } as any],
  // non-object ops entries: kept as given, length 1, act like {}
  nonObjNum: [5 as any],
  nonObjMid: [{ insert: "ab" }, 5 as any, { retain: 1 }],
  nonObjStr: ["s" as any, { retain: 1 }],
  nonObjArr: [[1] as any, { delete: 1 }],
  nonObjBool: [{ retain: 1 }, true as any, { insert: "q" }],
  nonObjAfterDel: [7 as any, { delete: 1 }],
};
const oddNames = Object.keys(odd);
const all: Record<string, Ops> = { ...pool, ...odd };
const attempt = <T>(f: () => T): { ok: true; v: T } | { ok: false } => {
  try { return { ok: true, v: f() }; } catch { return { ok: false }; }
};
// Emoji pool entries are left out: odd ops cut them in half, which would grow the counted
// half-emoji exceptions pinned in crates/delta/tests/fixtures.rs.
const allNames = Object.keys(all).filter((n) => n !== "emoji" && n !== "emojiRetain");
const record = (x: string, y: string, priorities: boolean[]) => {
  const A = d(all[x]!), B = d(all[y]!);
  const c = attempt(() => wire(A.compose(B)));
  if (c.ok) out.compose!.push({ name: `compose ${x} then ${y}`, input: { a: wire(A), b: wire(B) }, expected: c.v });
  for (const priority of priorities) {
    const t = attempt(() => wire(A.transform(B, priority)));
    if (t.ok) out.transform!.push({ name: `transform ${x} vs ${y} priority=${priority}`, input: { a: wire(A), b: wire(B), priority }, expected: t.v });
  }
};
for (const an of oddNames) {
  for (const [k, bn] of allNames.entries()) {
    const bothOdd = bn in odd;
    // odd x odd: every ordered pair; odd x plain: both orders, transform thinned.
    const thin = bothOdd || (oddNames.indexOf(an) + k) % 2 === 0 ? [true, false] : [];
    record(an, bn, thin);
    if (!bothOdd) record(bn, an, thin);
  }
  const A = d(odd[an]!);
  for (const index of [0, 1, 2, 3, 5, 8]) for (const priority of [true, false]) {
    const p = attempt(() => A.transformPosition(index, priority));
    if (p.ok) out.transform_position!.push({ name: `transformPosition ${an} index=${index} priority=${priority}`, input: { delta: wire(A), index, priority }, expected: p.v });
  }
}

// Raw JSON text (GUZ-718) for what JSON.stringify would already have changed in the
// pools above: whole-number floats, number formatting, numbers beyond f64 (JSON.parse
// reads them as Infinity), extra top-level keys. `new Delta(JSON.parse(text))` is what
// a browser does with a Delta from the wire. Written to parse.json; expected values
// are JSON text (exact number formatting), transformPosition results are String(x).
// Replayed by crates/delta/tests/fixtures.rs only.
const raw: Record<string, string> = {
  floatWhole: `{"ops":[{"insert":"a","attributes":{"header":1.0,"size":2e1,"w":-0.0}},{"retain":2.0},{"delete":3.00}]}`,
  floatAttrMerge: `{"ops":[{"insert":"a","attributes":{"h":1.0}},{"insert":"b","attributes":{"h":1}}]}`,
  floatRetainMerge: `{"ops":[{"retain":1.0,"attributes":{"x":[2.0]}},{"retain":1,"attributes":{"x":[2]}}]}`,
  floatFormat: `{"ops":[{"insert":{"image":"x","w":1e20,"h":1e21,"r":0.000001,"s":1e-7,"t":1.5e300,"u":-2.5e-9,"v":123.4560}}]}`,
  floatBig: `{"ops":[{"retain":1,"attributes":{"n":18446744073709551616,"m":-9223372036854775809}}]}`,
  extraKeys: `{"ops":[{"insert":"ab"},{"retain":1}],"extra":{"x":1},"y":[1e400]}`,
  extraKeysOnly: `{"ops":[],"note":"n"}`,
  nonObjectOps: `{"ops":[5,{"insert":"a"},"s",[1,{"insert":"x"}],true,{"retain":2}]}`,
  opsNumber: `{"ops":5}`,
  opsMissing: `{"x":[{"insert":"a"}]}`,
  topArray: `[{"insert":"a"},5,{"retain":1e400}]`,
  topArrayEmpty: `[]`,
  topScalar: `"ab"`,
  infRetain: `{"ops":[{"retain":1e400}]}`,
  infRetainAttr: `{"ops":[{"retain":1e400,"attributes":{"bold":true}}]}`,
  infRetainMid: `{"ops":[{"retain":1},{"retain":1e999},{"insert":"z"}]}`,
  infDelete: `{"ops":[{"insert":"x"},{"delete":-1e400}]}`,
  infDeletePos: `{"ops":[{"delete":1e400}]}`,
  infAttr: `{"ops":[{"insert":"ab","attributes":{"w":1e400}},{"insert":"c","attributes":{"w":null}}]}`,
  infAttrRetain: `{"ops":[{"retain":2,"attributes":{"w":-1e400,"bold":null}}]}`,
  infInsert: `{"ops":[{"insert":-1e400},{"insert":"k"}]}`,
  infEmbed: `{"ops":[{"insert":{"image":1e400,"list":[1e400,-1e400]}}]}`,
  // 1e15 is how Rust holds Infinity internally; as a literal it must stay a number.
  infMarkerLiteral: `{"ops":[{"retain":1e15},{"insert":"a","attributes":{"w":1.0e15,"v":-1E15,"u":1e18,"t":9007199254740993}}]}`,
};
const partners = ["text", "mixed", "retain3", "retainBold", "retainUnbold", "delete2", "insMid", "embedOnly", "zeroMix", "retainDelete", "insAttrArr", "nonObjMid"];
const P = (t: string) => new Delta(JSON.parse(t));
const parseOut: { name: string; input: Record<string, unknown>; expected: unknown }[] = [];
const texts: Record<string, string> = { ...raw };
for (const n of partners) texts[n] = JSON.stringify({ ops: all[n]! });
for (const [an, at] of Object.entries(raw)) {
  parseOut.push({ name: `parse ${an}`, input: { op: "parse", a: at }, expected: JSON.stringify(P(at)) });
  for (const index of [0, 1, 2, 3, 5]) for (const priority of [true, false]) {
    const p = attempt(() => String(P(at).transformPosition(index, priority)));
    if (p.ok) parseOut.push({ name: `transformPosition ${an} index=${index} priority=${priority}`, input: { op: "transform_position", a: at, index, priority }, expected: p.v });
  }
  for (const [bn, bt] of Object.entries(texts)) {
    for (const [x, xt, y, yt] of bn in raw ? [[an, at, bn, bt]] : [[an, at, bn, bt], [bn, bt, an, at]]) {
      const c = attempt(() => JSON.stringify(P(xt!).compose(P(yt!))));
      if (c.ok) parseOut.push({ name: `compose ${x} then ${y}`, input: { op: "compose", a: xt, b: yt }, expected: c.v });
      for (const priority of [true, false]) {
        const t = attempt(() => JSON.stringify(P(xt!).transform(P(yt!), priority)));
        if (t.ok) parseOut.push({ name: `transform ${x} vs ${y} priority=${priority}`, input: { op: "transform", a: xt, b: yt, priority }, expected: t.v });
      }
    }
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
writeFileSync(join(dir, "parse.json"), JSON.stringify(parseOut, null, 2) + "\n");
console.log(`parse: ${parseOut.length}`);
let total = parseOut.length;
for (const [op, cases] of Object.entries(out)) {
  writeFileSync(join(dir, `${op}.json`), JSON.stringify(cases, null, 2) + "\n");
  console.log(`${op}: ${cases.length}`);
  total += cases.length;
}
console.log(`total: ${total}`);
