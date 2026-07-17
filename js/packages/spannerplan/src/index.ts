import { normalizePlanInput as normalizePlanInputBrowser } from "./input-browser.js";
import { normalizePlanInput, normalizeStdinBytes } from "./input.js";
import type {
  BytesInput,
  Format,
  PlanInput,
  PlantreeConfig,
  PlantreeRowsResponse,
  RenderConfig,
  RenderMode,
  RenderResponse,
  RendertreeResponse,
} from "./types.js";
import {
  parsePlantreeRowsResponse,
  toPlantreeConfig,
} from "./plantree.js";
import { getBrowserWasm, getNodeWasm, isNodeRuntime } from "./wasm-node.js";

interface RawRenderResponse {
  output?: string;
  error?: string;
}

interface RawRendertreeResponse {
  output?: string;
  stderr?: string;
  error?: string;
  kind?: string;
}

function toRenderConfig(config: RenderConfig = {}): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  if (config.wrapWidth !== undefined) out.wrapWidth = config.wrapWidth;
  if (config.hangingIndent !== undefined) out.hangingIndent = config.hangingIndent;
  if (config.printSections !== undefined) out.printSections = config.printSections;
  if (config.showScalarVars !== undefined) out.showScalarVars = config.showScalarVars;
  if (config.resolveScalarVars !== undefined) {
    out.resolveScalarVars = config.resolveScalarVars;
  }
  if (config.resolveScalarVarsRecursive !== undefined) {
    out.resolveScalarVarsRecursive = config.resolveScalarVarsRecursive;
  }
  if (config.disallowUnknownStats !== undefined) {
    out.disallowUnknownStats = config.disallowUnknownStats;
  }
  return out;
}

function parseRenderResponse(raw: unknown): RenderResponse {
  const value = raw as RawRenderResponse;
  if (value.error) {
    return { error: value.error };
  }
  if (typeof value.output === "string") {
    return { output: value.output };
  }
  return { error: "unexpected WASM render response" };
}

function parseRendertreeResponse(raw: unknown): RendertreeResponse {
  const value = raw as RawRendertreeResponse;
  const kind = value.kind ?? "failed";
  if (kind === "rendered" && typeof value.output === "string") {
    return { kind: "rendered", output: value.output };
  }
  if (kind === "help") {
    return { kind: "help", stderr: value.stderr ?? "" };
  }
  if (kind === "usage") {
    return {
      kind: "usage",
      stderr: value.stderr ?? "",
      error: value.error ?? "usage error",
    };
  }
  return {
    kind: "failed",
    stderr: value.stderr ?? "",
    error: value.error ?? "render failed",
  };
}

function renderTreeTableNode(
  plan: string | Uint8Array | Record<string, unknown>,
  mode: RenderMode,
  format: Format,
  config: RenderConfig,
): RenderResponse {
  const wasm = getNodeWasm();
  const normalized = normalizePlanInput(plan);
  const args = [
    normalized,
    mode,
    format,
    toRenderConfig(config),
  ];
  return parseRenderResponse(wasm.spannerplanRenderTreeTable(args));
}

function plantreeRowsNode(
  plan: string | Uint8Array | Record<string, unknown>,
  format: Format,
  config: PlantreeConfig,
): PlantreeRowsResponse {
  const wasm = getNodeWasm();
  const normalized = normalizePlanInput(plan);
  if (normalized instanceof Uint8Array) {
    return parsePlantreeRowsResponse(
      wasm.spannerplanPlantreeRowsWire(normalized, format, toPlantreeConfig(config)),
    );
  }
  return parsePlantreeRowsResponse(
    wasm.spannerplanPlantreeRows([normalized, format, toPlantreeConfig(config)]),
  );
}

async function plantreeRowsBrowser(
  plan: string | Uint8Array | Record<string, unknown>,
  format: Format,
  config: PlantreeConfig,
): Promise<PlantreeRowsResponse> {
  const wasm = await getBrowserWasm();
  const normalized = normalizePlanInputBrowser(plan);
  if (normalized instanceof Uint8Array) {
    return parsePlantreeRowsResponse(
      wasm.spannerplanPlantreeRowsWire(normalized, format, toPlantreeConfig(config)),
    );
  }
  return parsePlantreeRowsResponse(
    wasm.spannerplanPlantreeRows([normalized, format, toPlantreeConfig(config)]),
  );
}

async function renderTreeTableBrowser(
  plan: string | Uint8Array | Record<string, unknown>,
  mode: RenderMode,
  format: Format,
  config: RenderConfig,
): Promise<RenderResponse> {
  const wasm = await getBrowserWasm();
  const normalized = normalizePlanInputBrowser(plan);
  if (normalized instanceof Uint8Array) {
    const raw = wasm.spannerplanRenderTreeTableWire(
      normalized,
      mode,
      format,
      toRenderConfig(config),
    );
    return parseRenderResponse(raw);
  }
  const args = [normalized, mode, format, toRenderConfig(config)];
  return parseRenderResponse(wasm.spannerplanRenderTreeTable(args));
}

/**
 * Render a Spanner query plan using the reference API (matches Go
 * `RenderTreeTableWithConfig` / Rust `render_tree_table_with_config`).
 */
export function renderTreeTable(
  plan: PlanInput,
  mode: RenderMode = "AUTO",
  format: Format = "CURRENT",
  config: RenderConfig = {},
): RenderResponse | Promise<RenderResponse> {
  if (isNodeRuntime()) {
    return renderTreeTableNode(plan, mode, format, config);
  }
  return renderTreeTableBrowser(plan, mode, format, config);
}

/**
 * Render protobuf wire-encoded plan nodes (reference API).
 */
export function renderTreeTableWire(
  planWire: Uint8Array,
  mode: RenderMode = "AUTO",
  format: Format = "CURRENT",
  config: RenderConfig = {},
): RenderResponse | Promise<RenderResponse> {
  const cfg = toRenderConfig(config);
  if (isNodeRuntime()) {
    const wasm = getNodeWasm();
    return parseRenderResponse(
      wasm.spannerplanRenderTreeTableWire(planWire, mode, format, cfg),
    );
  }
  return getBrowserWasm().then((wasm) =>
    parseRenderResponse(
      wasm.spannerplanRenderTreeTableWire(planWire, mode, format, cfg),
    ),
  );
}

/**
 * Return structured Plantree rows without parsing formatted table text.
 * Node.js returns synchronously; browser hosts return a Promise.
 */
export function plantreeRows(
  plan: PlanInput,
  format: Format = "CURRENT",
  config: PlantreeConfig = {},
): PlantreeRowsResponse | Promise<PlantreeRowsResponse> {
  if (isNodeRuntime()) {
    return plantreeRowsNode(plan, format, config);
  }
  return plantreeRowsBrowser(plan, format, config);
}

/** Return structured Plantree rows from protobuf wire plan bytes. */
export function plantreeRowsWire(
  planWire: Uint8Array,
  format: Format = "CURRENT",
  config: PlantreeConfig = {},
): PlantreeRowsResponse | Promise<PlantreeRowsResponse> {
  const cfg = toPlantreeConfig(config);
  if (isNodeRuntime()) {
    return parsePlantreeRowsResponse(
      getNodeWasm().spannerplanPlantreeRowsWire(planWire, format, cfg),
    );
  }
  return getBrowserWasm().then((wasm) =>
    parsePlantreeRowsResponse(
      wasm.spannerplanPlantreeRowsWire(planWire, format, cfg),
    ),
  );
}

/**
 * Render stdin bytes with `rendertree` CLI semantics (matches Go/Rust CLI).
 * Node only.
 */
export function renderRendertree(
  input: BytesInput,
  args: string[] = [],
): RendertreeResponse {
  if (!isNodeRuntime()) {
    throw new Error("renderRendertree is only supported in Node.js");
  }
  const wasm = getNodeWasm();
  const bytes = normalizeStdinBytes(input);
  return parseRendertreeResponse(wasm.spannerplanRenderRendertree(bytes, args));
}

/** Convenience: reference render or throw on error. */
export async function renderTreeTableOrThrow(
  plan: PlanInput,
  mode: RenderMode = "AUTO",
  format: Format = "CURRENT",
  config: RenderConfig = {},
): Promise<string> {
  const result = await renderTreeTable(plan, mode, format, config);
  if ("error" in result) {
    throw new Error(result.error);
  }
  return result.output;
}

/** Convenience: structured Plantree rows or throw on error. */
export async function plantreeRowsOrThrow(
  plan: PlanInput,
  format: Format = "CURRENT",
  config: PlantreeConfig = {},
) {
  const result = await plantreeRows(plan, format, config);
  if ("error" in result) {
    throw new Error(result.error);
  }
  return result.rows;
}

/** Convenience: rendertree render or throw on error. */
export function renderRendertreeOrThrow(
  input: BytesInput,
  args: string[] = [],
): string {
  const result = renderRendertree(input, args);
  if (result.kind === "rendered") {
    return result.output;
  }
  if (result.kind === "help") {
    throw new Error(result.stderr);
  }
  const message = result.error || result.stderr || "rendertree failed";
  const err = new Error(message);
  if (result.kind === "usage") {
    (err as Error & { exitCode: number }).exitCode = 2;
  }
  throw err;
}

export { normalizePlanInput, normalizeStdinBytes } from "./input.js";
export { getBrowserWasm, getNodeWasm, isNodeRuntime } from "./wasm-node.js";
export * from "./types.js";
