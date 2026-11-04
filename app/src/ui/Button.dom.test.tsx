import { afterEach, expect, test } from "bun:test";
import { cleanup, click, render } from "../test/dom";
import { Button } from "./Button";

afterEach(cleanup);

test("disabled Button blocks clicks", async () => {
  let clicks = 0;
  const container = await render(<><Button disabled onClick={() => clicks++}>Disabled</Button><Button onClick={() => clicks++}>Enabled</Button></>);
  const buttons = container.querySelectorAll("button");
  await click(buttons[0]);
  expect(clicks).toBe(0);
  await click(buttons[1]);
  expect(clicks).toBe(1);
});
