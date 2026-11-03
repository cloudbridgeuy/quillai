import { expect, test } from "bun:test";
import { scriptNotice } from "./script_notice";

test("script notices preserve priority and assign the correct tone", () => {
  expect(scriptNotice("Could not add scene", true, "Other notice")).toEqual({ text: "Could not add scene", tone: "error" });
  expect(scriptNotice(null, true, "Other notice")).toEqual({ text: "Scenes changed; reload", tone: "warning" });
  expect(scriptNotice(null, false, "Other notice")).toEqual({ text: "Other notice", tone: "warning" });
  expect(scriptNotice(null, false, null)).toBeNull();
});
