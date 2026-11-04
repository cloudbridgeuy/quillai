import { afterEach, expect, test } from "bun:test";
import { cleanup, click, render } from "../test/dom";
import { ActivityList } from "./ActivityList";

const originalFetch = globalThis.fetch;
afterEach(async () => { await cleanup(); globalThis.fetch = originalFetch; });

function respond(response: () => Promise<Response>) {
  globalThis.fetch = Object.assign(response, { preconnect: originalFetch.preconnect });
}

test("shows day headers and opens activity scope", async () => {
  const now = new Date();
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1, 12);
  const earlier = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 2, 12);
  respond(async () => Response.json([
    { pieceId: 1, universeId: 2, title: "Today story", label: "piece", format: "prose", parentId: null, changedAt: now.toISOString(), chapter: null },
    { pieceId: 4, universeId: 2, title: "Old story", label: "piece", format: "prose", parentId: null, changedAt: earlier.toISOString(), chapter: null },
    { pieceId: 3, universeId: 2, title: "Yesterday story", label: "piece", format: "prose", parentId: null, changedAt: yesterday.toISOString(), chapter: null },
  ]));
  let opened: unknown;
  const container = await render(<ActivityList revision="0" onOpen={scope => { opened = scope; }} />);
  expect(container.textContent).toContain("Recent activity");
  expect([...container.querySelectorAll("h3")].map(heading => heading.textContent)).toEqual(["Today", "Yesterday", "Earlier"]);
  await click(container.querySelector("button")!);
  expect(opened).toEqual({ universeId: 2, pieceId: 1 });
});

test("hides an empty list and reports failure", async () => {
  respond(async () => Response.json([]));
  const empty = await render(<ActivityList revision="0" onOpen={() => {}} />);
  expect(empty.textContent).toBe("");
  respond(async () => { throw new Error("offline"); });
  const failed = await render(<ActivityList revision="0" onOpen={() => {}} />);
  expect(failed.textContent).toContain("Could not load recent activity.");
});
