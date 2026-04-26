import { expect, test } from "bun:test";

import { finalizeAiStreamRemainder, parseAiStreamChunk } from "./ai_stream";

function expectParseFailure(result: ReturnType<typeof parseAiStreamChunk>) {
  expect(result.ok).toBe(false);
  if (result.ok) {
    throw new Error("expected parse failure");
  }

  return result.error;
}

test("parseAiStreamChunk parses one complete NDJSON line", () => {
  const result = parseAiStreamChunk(
    '{"type":"done"}\n',
  );

  expect(result).toEqual({
    ok: true,
    events: [{ type: "done" }],
    remainder: "",
  });
});

test("parseAiStreamChunk parses multiple complete lines from one chunk", () => {
  const result = parseAiStreamChunk(
    '{"type":"delta","delta":{"ops":[{"insert":"Hello"}]}}\n{"type":"cancelled"}\n',
  );

  expect(result).toEqual({
    ok: true,
    events: [
      { type: "delta", delta: { ops: [{ insert: "Hello" }] } },
      { type: "cancelled" },
    ],
    remainder: "",
  });
});

test("parseAiStreamChunk buffers a split line across chunk boundaries", () => {
  const first = parseAiStreamChunk('{"type":"err');
  expect(first).toEqual({ ok: true, events: [], remainder: '{"type":"err' });

  const second = parseAiStreamChunk('or","error":"boom"}\n', first.remainder);
  expect(second).toEqual({
    ok: true,
    events: [{ type: "error", error: "boom" }],
    remainder: "",
  });
});

test("parseAiStreamChunk returns a typed parse failure for malformed JSON", () => {
  const result = parseAiStreamChunk('{"type":"done"\n');

  const error = expectParseFailure(result);

  expect(result.events).toEqual([]);
  expect(error.code).toBe("invalid_json");
  expect(error.line).toBe('{"type":"done"');
});

test("parseAiStreamChunk returns a typed parse failure for unsupported events", () => {
  const result = parseAiStreamChunk('{"type":"mystery"}\n');

  const error = expectParseFailure(result);

  expect(error.code).toBe("invalid_event");
  expect(error.message).toContain("unsupported stream event type");
});

test("parseAiStreamChunk parses done error and cancelled variants", () => {
  const result = parseAiStreamChunk(
    '{"type":"done"}\n{"type":"error","error":"boom"}\n{"type":"cancelled"}\n',
  );

  expect(result).toEqual({
    ok: true,
    events: [
      { type: "done" },
      { type: "error", error: "boom" },
      { type: "cancelled" },
    ],
    remainder: "",
  });
});

test("parseAiStreamChunk skips blank lines", () => {
  const result = parseAiStreamChunk('\n  \n{"type":"done"}\n\t\n');

  expect(result).toEqual({
    ok: true,
    events: [{ type: "done" }],
    remainder: "",
  });
});

test("parseAiStreamChunk rejects a non-object event", () => {
  const result = parseAiStreamChunk('true\n');

  const error = expectParseFailure(result);

  expect(error).toEqual({
    code: "invalid_event",
    message: "stream event must be an object",
    line: "true",
  });
});

test("parseAiStreamChunk rejects events with missing or non-string types", () => {
  const missingType = parseAiStreamChunk('{}\n');
  expect(expectParseFailure(missingType)).toEqual({
    code: "invalid_event",
    message: "stream event type must be a string",
    line: "{}",
  });

  const numericType = parseAiStreamChunk('{"type":123}\n');
  expect(expectParseFailure(numericType)).toEqual({
    code: "invalid_event",
    message: "stream event type must be a string",
    line: '{"type":123}',
  });
});

test("parseAiStreamChunk rejects delta events without a delta document", () => {
  const result = parseAiStreamChunk('{"type":"delta","delta":"nope"}\n');

  const error = expectParseFailure(result);

  expect(error).toEqual({
    code: "invalid_event",
    message: "delta event must contain a delta document",
    line: '{"type":"delta","delta":"nope"}',
  });
});

test("parseAiStreamChunk rejects delta events with malformed ops", () => {
  const nullOp = parseAiStreamChunk('{"type":"delta","delta":{"ops":[null]}}\n');
  expect(expectParseFailure(nullOp)).toEqual({
    code: "invalid_event",
    message: "delta event must contain a delta document",
    line: '{"type":"delta","delta":{"ops":[null]}}',
  });

  const invalidInsert = parseAiStreamChunk(
    '{"type":"delta","delta":{"ops":[{"insert":5}]}}\n',
  );
  expect(expectParseFailure(invalidInsert)).toEqual({
    code: "invalid_event",
    message: "delta event must contain a delta document",
    line: '{"type":"delta","delta":{"ops":[{"insert":5}]}}',
  });

  const conflictingOp = parseAiStreamChunk(
    '{"type":"delta","delta":{"ops":[{"insert":"x","delete":1}]}}\n',
  );
  expect(expectParseFailure(conflictingOp)).toEqual({
    code: "invalid_event",
    message: "delta event must contain a delta document",
    line: '{"type":"delta","delta":{"ops":[{"insert":"x","delete":1}]}}',
  });
});

test("parseAiStreamChunk rejects error events without a string message", () => {
  const result = parseAiStreamChunk('{"type":"error","error":null}\n');

  const error = expectParseFailure(result);

  expect(error).toEqual({
    code: "invalid_event",
    message: "error event must contain an error message",
    line: '{"type":"error","error":null}',
  });
});

test("finalizeAiStreamRemainder parses a final line without a trailing newline", () => {
  const chunk = parseAiStreamChunk('{"type":"done"}');
  expect(chunk).toEqual({ ok: true, events: [], remainder: '{"type":"done"}' });

  expect(finalizeAiStreamRemainder(chunk.remainder)).toEqual({
    ok: true,
    events: [{ type: "done" }],
    remainder: "",
  });
});

test("finalizeAiStreamRemainder returns a typed parse failure for malformed final JSON", () => {
  const result = finalizeAiStreamRemainder('{"type":"done"');

  const error = expectParseFailure(result);

  expect(error.code).toBe("invalid_json");
  expect(error.line).toBe('{"type":"done"');
  expect(result.remainder).toBe("");
});

test("finalizeAiStreamRemainder ignores blank final remainder", () => {
  expect(finalizeAiStreamRemainder("   \t ")).toEqual({
    ok: true,
    events: [],
    remainder: "",
  });
});
