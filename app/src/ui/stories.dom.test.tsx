import { afterEach, expect, test } from "bun:test";
import { createElement, type ComponentType, type ReactNode } from "react";
import { cleanup, render } from "../test/dom";

type Args = Record<string, unknown>;
type Story = {
  name?: string;
  args?: Args;
  render?: (args: Args) => ReactNode;
  play?: (context: { canvasElement: HTMLElement; args: Args }) => Promise<void> | void;
};
type StoryModule = { default: { title: string; component?: ComponentType<Args>; args?: Args } } & Record<string, Story>;

afterEach(cleanup);

const files = [...new Bun.Glob("*.stories.tsx").scanSync(import.meta.dir)].sort();

test("ui stories exist", () => {
  expect(files.length).toBeGreaterThan(0);
});

for (const file of files) {
  const { default: meta, ...stories } = (await import(`./${file}`)) as StoryModule;
  for (const [exportName, story] of Object.entries(stories)) {
    test(`${meta.title} / ${story.name ?? exportName}`, async () => {
      const args = { ...meta.args, ...story.args };
      const canvasElement = await render(story.render ? story.render(args) : createElement(meta.component!, args));
      Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: false });
      try {
        await story.play?.({ canvasElement, args });
      } finally {
        Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
      }
    });
  }
}
