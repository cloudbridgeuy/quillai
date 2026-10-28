import { expect, test } from "bun:test";
import { getEditFailurePresentation } from "./edit_failure";

test("presents malformed edit responses with a friendly message and technical title", () => {
  for (const error of [
    "AI edit response did not include a <replacement> tag.",
    "AI edit response ended before the closing </replacement> tag.",
  ]) {
    expect(getEditFailurePresentation(error)).toEqual({
      label: "The model didn't return an edit. Try again or switch models.",
      title: error,
    });
  }
});

test("preserves other edit errors", () => {
  for (const error of ["Mock stream failure", ""]) {
    expect(getEditFailurePresentation(error)).toEqual({ label: error });
  }
});
