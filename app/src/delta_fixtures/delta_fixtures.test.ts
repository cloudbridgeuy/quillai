import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const dir = join(import.meta.dir, "../../../fixtures/delta");
type Case = { name: string; input: any; expected: any };
const load = (op: string): Case[] => JSON.parse(readFileSync(join(dir, `${op}.json`), "utf8"));
const D = (w: { ops: any[] }) => new Delta(w.ops);
const wire = (x: Delta) => JSON.parse(JSON.stringify({ ops: x.ops }));

const replay: Record<string, (i: any) => unknown> = {
  compose: (i) => wire(D(i.a).compose(D(i.b))),
  transform: (i) => wire(D(i.a).transform(D(i.b), i.priority)),
  transform_position: (i) => D(i.delta).transformPosition(i.index, i.priority),
  invert: (i) => wire(D(i.change).invert(D(i.base))),
  diff: (i) => wire(D(i.a).diff(D(i.b), i.cursor)),
};

let total = 0;
for (const [op, fn] of Object.entries(replay)) {
  const cases = load(op);
  total += cases.length;
  describe(`delta fixtures: ${op}`, () => {
    test("has cases", () => expect(cases.length).toBeGreaterThan(0));
    for (const c of cases) test(c.name, () => expect(fn(c.input)).toEqual(c.expected));
  });
}
test("at least 100 delta fixtures", () => expect(total).toBeGreaterThanOrEqual(100));
