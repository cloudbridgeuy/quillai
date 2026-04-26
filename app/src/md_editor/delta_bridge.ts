import type { DeltaDocument, DeltaOperation, DeltaAttributes } from "./ai_contract";
import type { DeltaT } from "./types";

type QuillOpLike = {
  insert?: unknown;
  delete?: unknown;
  retain?: unknown;
  attributes?: Record<string, unknown> | null;
};

type QuillDeltaLike = {
  ops?: QuillOpLike[];
};

type QuillDeltaDocument = {
  ops: Array<{
    insert?: string | Record<string, unknown>;
    delete?: number;
    retain?: number;
    attributes?: Record<string, unknown>;
  }>;
};

export function toContractDeltaDocument(delta: DeltaT | QuillDeltaLike): DeltaDocument {
  return {
    ops: (delta.ops ?? []).flatMap((op) => {
      const contractOp = toContractDeltaOperation(op);
      return hasDeltaOperationKind(contractOp) ? [contractOp] : [];
    }),
  };
}

export function toQuillDeltaDocument(document: DeltaDocument): QuillDeltaDocument {
  return {
    ops: document.ops.map(toQuillDeltaOperation),
  };
}

function toContractDeltaOperation(op: QuillOpLike): DeltaOperation {
  const attributes = toContractAttributes(op.attributes);
  const insert = isInsertValue(op.insert) ? op.insert : undefined;
  const del = typeof op.delete === "number" ? op.delete : undefined;
  const retain = typeof op.retain === "number" ? op.retain : undefined;

  return {
    ...(insert !== undefined ? { insert } : {}),
    ...(del !== undefined ? { delete: del } : {}),
    ...(retain !== undefined ? { retain } : {}),
    ...(attributes ? { attributes } : {}),
  };
}

function hasDeltaOperationKind(op: DeltaOperation): boolean {
  return op.insert !== undefined || op.delete !== undefined || op.retain !== undefined;
}

function toQuillDeltaOperation(op: DeltaOperation): QuillDeltaDocument["ops"][number] {
  const attributes = toQuillAttributes(op.attributes);

  return {
    insert: op.insert,
    delete: op.delete,
    retain: op.retain,
    ...(attributes ? { attributes } : {}),
  };
}

function toContractAttributes(
  attributes: Record<string, unknown> | null | undefined,
): DeltaAttributes | undefined {
  if (!attributes) {
    return undefined;
  }

  const result: DeltaAttributes = {};

  for (const [key, value] of Object.entries(attributes)) {
    if (
      value === null ||
      typeof value === "string" ||
      typeof value === "number" ||
      typeof value === "boolean"
    ) {
      result[key] = value;
    }
  }

  return Object.keys(result).length > 0 ? result : undefined;
}

function toQuillAttributes(
  attributes: DeltaAttributes | null | undefined,
): Record<string, unknown> | undefined {
  if (!attributes) {
    return undefined;
  }

  const entries = Object.entries(attributes).filter(([, value]) => value !== null);
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function isInsertValue(value: unknown): value is string | Record<string, unknown> {
  return typeof value === "string" || isRecord(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
