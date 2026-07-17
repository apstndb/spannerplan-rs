import type {
  InternalPlantreeConfigV1Alpha2,
  InternalPlantreeChildLinkV1Alpha2,
  InternalPlantreeRowV1Alpha2,
  InternalPlantreeRowsResponseV1Alpha2,
} from "./types.js";

interface RawPlantreeRowsResponse {
  contractVersion?: unknown;
  rows?: unknown;
  error?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function parsePlantreeChildLink(value: unknown): InternalPlantreeChildLinkV1Alpha2 | null {
  if (
    !isRecord(value) ||
    typeof value.type !== "string" ||
    typeof value.variable !== "string" ||
    typeof value.description !== "string" ||
    typeof value.displayName !== "string" ||
    typeof value.childIndex !== "number" ||
    !Number.isInteger(value.childIndex) ||
    typeof value.isPredicate !== "boolean"
  ) {
    return null;
  }

  return {
    type: value.type,
    variable: value.variable,
    description: value.description,
    displayName: value.displayName,
    childIndex: value.childIndex,
    isPredicate: value.isPredicate,
  };
}

function parsePlantreeRow(value: unknown): InternalPlantreeRowV1Alpha2 | null {
  if (
    !isRecord(value) ||
    typeof value.rowId !== "string" ||
    !(typeof value.parentRowId === "string" || value.parentRowId === null) ||
    typeof value.nodeId !== "number" ||
    !Number.isInteger(value.nodeId) ||
    typeof value.treePart !== "string" ||
    typeof value.nodeText !== "string" ||
    typeof value.displayName !== "string" ||
    !isStringArray(value.predicates) ||
    !Array.isArray(value.scalarChildLinks)
  ) {
    return null;
  }

  const scalarChildLinks: InternalPlantreeChildLinkV1Alpha2[] = [];
  for (const childLink of value.scalarChildLinks) {
    const parsed = parsePlantreeChildLink(childLink);
    if (!parsed) return null;
    scalarChildLinks.push(parsed);
  }

  return {
    rowId: value.rowId,
    parentRowId: value.parentRowId,
    nodeId: value.nodeId,
    treePart: value.treePart,
    nodeText: value.nodeText,
    displayName: value.displayName,
    predicates: [...value.predicates],
    scalarChildLinks,
  };
}

/** Convert the deliberately small structured-Plantree config to WASM JSON. */
export function toInternalPlantreeConfigV1Alpha2(
  config: InternalPlantreeConfigV1Alpha2 = {},
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  if (config.wrapWidth !== undefined) out.wrapWidth = config.wrapWidth;
  if (config.hangingIndent !== undefined) out.hangingIndent = config.hangingIndent;
  if (config.disallowUnknownStats !== undefined) {
    out.disallowUnknownStats = config.disallowUnknownStats;
  }
  return out;
}

/**
 * Validate the outer WASM response envelope without trying to reconstruct or
 * parse the formatted Plantree text. It validates the versioned DTO fields,
 * but deliberately does not infer any structure from rendered strings.
 */
export function parseInternalPlantreeRowsResponseV1Alpha2(
  raw: unknown,
): InternalPlantreeRowsResponseV1Alpha2 {
  if (!raw || typeof raw !== "object") {
    return { error: "unexpected WASM Plantree response" };
  }

  const value = raw as RawPlantreeRowsResponse;
  if ("error" in value) {
    if (typeof value.error === "string") {
      return { error: value.error };
    }
    return { error: "unexpected WASM Plantree error response" };
  }
  if (value.contractVersion !== 2) {
    return { error: "unsupported WASM Plantree contract version" };
  }
  if (!Array.isArray(value.rows)) {
    return { error: "unexpected WASM Plantree rows response" };
  }
  const rows: InternalPlantreeRowV1Alpha2[] = [];
  for (const row of value.rows) {
    const parsed = parsePlantreeRow(row);
    if (!parsed) {
      return { error: "unexpected WASM Plantree row response" };
    }
    rows.push(parsed);
  }

  return {
    contractVersion: 2,
    rows,
  };
}
