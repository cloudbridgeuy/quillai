import { test, expect } from "bun:test";
import { formatResponseBody } from "./APITester";

test("success response returns pretty JSON", () => {
  const result = formatResponseBody(true, 200, "OK", { message: "hi" });
  expect(result).toBe(JSON.stringify({ message: "hi" }, null, 2));
});

test("error response with API error shape returns formatted error", () => {
  const result = formatResponseBody(false, 500, "Internal Server Error", {
    error: "sqlx error",
    code: "SQLX",
  });
  expect(result).toBe("Error [SQLX]: sqlx error");
});

test("error response with null body returns status line", () => {
  const result = formatResponseBody(false, 404, "Not Found", null);
  expect(result).toBe("404 Not Found");
});

test("error response with unexpected shape returns status line", () => {
  const result = formatResponseBody(false, 500, "Internal Server Error", {
    unexpected: "shape",
  });
  expect(result).toBe("500 Internal Server Error");
});
