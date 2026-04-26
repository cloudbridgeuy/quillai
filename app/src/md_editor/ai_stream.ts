import type { AiStreamEvent, DeltaAttributes, DeltaDocument, DeltaInsert, DeltaOperation } from "./ai_contract";

export type AiStreamParseError = {
  code: "invalid_json" | "invalid_event";
  message: string;
  line: string;
};

export type ParseAiStreamChunkSuccess = {
  ok: true;
  events: AiStreamEvent[];
  remainder: string;
};

export type ParseAiStreamChunkFailure = {
  ok: false;
  events: AiStreamEvent[];
  remainder: string;
  error: AiStreamParseError;
};

export type ParseAiStreamChunkResult =
  | ParseAiStreamChunkSuccess
  | ParseAiStreamChunkFailure;

export function parseAiStreamChunk(
  chunk: string,
  remainder = "",
): ParseAiStreamChunkResult {
  const combined = `${remainder}${chunk}`;
  const lines = combined.split("\n");
  const nextRemainder = lines.pop() ?? "";
  const events: AiStreamEvent[] = [];

  for (const rawLine of lines) {
    const parsedLine = parseLine(rawLine);
    if (!parsedLine) {
      continue;
    }

    if (!parsedLine.ok) {
      return {
        ok: false,
        events,
        remainder: nextRemainder,
        error: parsedLine.error,
      };
    }

    const event = toAiStreamEvent(parsedLine.value, parsedLine.line);
    if (!event.ok) {
      return {
        ok: false,
        events,
        remainder: nextRemainder,
        error: event.error,
      };
    }

    events.push(event.event);
  }

  return {
    ok: true,
    events,
    remainder: nextRemainder,
  };
}

export function finalizeAiStreamRemainder(
  remainder: string,
): ParseAiStreamChunkResult {
  if (remainder === "") {
    return {
      ok: true,
      events: [],
      remainder: "",
    };
  }

  const parsedLine = parseLine(remainder);
  if (!parsedLine) {
    return {
      ok: true,
      events: [],
      remainder: "",
    };
  }

  if (!parsedLine.ok) {
    return {
      ok: false,
      events: [],
      remainder: "",
      error: parsedLine.error,
    };
  }

  const event = toAiStreamEvent(parsedLine.value, parsedLine.line);
  if (!event.ok) {
    return {
      ok: false,
      events: [],
      remainder: "",
      error: event.error,
    };
  }

  return {
    ok: true,
    events: [event.event],
    remainder: "",
  };
}

function parseLine(
  rawLine: string,
): { ok: true; line: string; value: unknown } | { ok: false; error: AiStreamParseError } | null {
  const line = rawLine.trim();
  if (line === "") {
    return null;
  }

  try {
    return {
      ok: true,
      line,
      value: JSON.parse(line),
    };
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "invalid_json",
        message: error instanceof Error ? error.message : "Invalid JSON",
        line,
      },
    };
  }
}

function toAiStreamEvent(
  value: unknown,
  line: string,
): { ok: true; event: AiStreamEvent } | { ok: false; error: AiStreamParseError } {
  if (!value || typeof value !== "object") {
    return invalidEvent("stream event must be an object", line);
  }

  const type = (value as { type?: unknown }).type;
  if (typeof type !== "string") {
    return invalidEvent("stream event type must be a string", line);
  }

  switch (type) {
    case "delta": {
      const delta = (value as { delta?: unknown }).delta;
      if (!isDeltaDocument(delta)) {
        return invalidEvent("delta event must contain a delta document", line);
      }

      return { ok: true, event: { type: "delta", delta } };
    }

    case "done":
      return { ok: true, event: { type: "done" } };

    case "error": {
      const error = (value as { error?: unknown }).error;
      if (typeof error !== "string") {
        return invalidEvent("error event must contain an error message", line);
      }

      return { ok: true, event: { type: "error", error } };
    }

    case "cancelled":
      return { ok: true, event: { type: "cancelled" } };

    default:
      return invalidEvent(`unsupported stream event type: ${type}`, line);
  }
}

function invalidEvent(message: string, line: string) {
  return {
    ok: false as const,
    error: {
      code: "invalid_event" as const,
      message,
      line,
    },
  };
}

function isDeltaDocument(value: unknown): value is DeltaDocument {
  if (!value || typeof value !== "object") {
    return false;
  }

  const ops = (value as { ops?: unknown }).ops;
  return Array.isArray(ops) && ops.every(isDeltaOperation);
}

function isDeltaOperation(value: unknown): value is DeltaOperation {
  if (!isRecord(value)) {
    return false;
  }

  const hasInsert = value.insert !== undefined;
  const hasDelete = value.delete !== undefined;
  const hasRetain = value.retain !== undefined;

  if (Number(hasInsert) + Number(hasDelete) + Number(hasRetain) !== 1) {
    return false;
  }

  if (hasInsert && !isDeltaInsert(value.insert)) {
    return false;
  }

  if (hasDelete && !isPositiveInteger(value.delete)) {
    return false;
  }

  if (hasRetain && !isPositiveInteger(value.retain)) {
    return false;
  }

  return value.attributes === undefined || value.attributes === null || isDeltaAttributes(value.attributes);
}

function isDeltaInsert(value: unknown): value is DeltaInsert {
  return typeof value === "string" || isRecord(value);
}

function isDeltaAttributes(value: unknown): value is DeltaAttributes {
  if (!isRecord(value)) {
    return false;
  }

  return Object.values(value).every(
    (attribute) =>
      attribute === null ||
      typeof attribute === "string" ||
      typeof attribute === "number" ||
      typeof attribute === "boolean",
  );
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
