import type {
  Format,
  InternalPlantreeConfigV1Alpha2,
  InternalPlantreeRowsResponseV1Alpha2,
  PlanInput,
  RenderConfig,
  RenderMode,
  RenderResponse,
} from "./types.js";
import {
  parseInternalPlantreeRowsResponseV1Alpha2,
  toInternalPlantreeConfigV1Alpha2,
} from "./plantree.js";
import { normalizePlanInput } from "./input-browser.js";
import { getBrowserWasm } from "./wasm-browser.js";

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
  const value = raw as { output?: string; error?: string };
  if (value.error) {
    return { error: value.error };
  }
  if (typeof value.output === "string") {
    return { output: value.output };
  }
  return { error: "unexpected WASM render response" };
}

/**
 * Render a Spanner query plan using the reference API (browser / bundler entry).
 */
export async function renderTreeTable(
  plan: PlanInput,
  mode: RenderMode = "AUTO",
  format: Format = "CURRENT",
  config: RenderConfig = {},
): Promise<RenderResponse> {
  const wasm = await getBrowserWasm();
  const normalized = normalizePlanInput(plan);
  if (normalized instanceof Uint8Array) {
    return parseRenderResponse(
      wasm.spannerplanRenderTreeTableWire(
        normalized,
        mode,
        format,
        toRenderConfig(config),
      ),
    );
  }
  return parseRenderResponse(
    wasm.spannerplanRenderTreeTable([
      normalized,
      mode,
      format,
      toRenderConfig(config),
    ]),
  );
}

/** Render protobuf wire-encoded plan nodes (browser / bundler entry). */
export async function renderTreeTableWire(
  planWire: Uint8Array,
  mode: RenderMode = "AUTO",
  format: Format = "CURRENT",
  config: RenderConfig = {},
): Promise<RenderResponse> {
  const wasm = await getBrowserWasm();
  return parseRenderResponse(
    wasm.spannerplanRenderTreeTableWire(
      planWire,
      mode,
      format,
      toRenderConfig(config),
    ),
  );
}

/** @internal Bundled viewer Plantree v1alpha2 contract. */
export async function internalPlantreeRowsV1Alpha2(
  plan: PlanInput,
  format: Format = "CURRENT",
  config: InternalPlantreeConfigV1Alpha2 = {},
): Promise<InternalPlantreeRowsResponseV1Alpha2> {
  const wasm = await getBrowserWasm();
  const normalized = normalizePlanInput(plan);
  if (normalized instanceof Uint8Array) {
    return parseInternalPlantreeRowsResponseV1Alpha2(
      wasm.spannerplanInternalPlantreeRowsV1Alpha2Wire(
        normalized,
        format,
        toInternalPlantreeConfigV1Alpha2(config),
      ),
    );
  }
  return parseInternalPlantreeRowsResponseV1Alpha2(
    wasm.spannerplanInternalPlantreeRowsV1Alpha2([
      normalized,
      format,
      toInternalPlantreeConfigV1Alpha2(config),
    ]),
  );
}

/** @internal Wire-input variant of the bundled viewer contract. */
export async function internalPlantreeRowsV1Alpha2Wire(
  planWire: Uint8Array,
  format: Format = "CURRENT",
  config: InternalPlantreeConfigV1Alpha2 = {},
): Promise<InternalPlantreeRowsResponseV1Alpha2> {
  const wasm = await getBrowserWasm();
  return parseInternalPlantreeRowsResponseV1Alpha2(
    wasm.spannerplanInternalPlantreeRowsV1Alpha2Wire(
      planWire,
      format,
      toInternalPlantreeConfigV1Alpha2(config),
    ),
  );
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

/** @internal Convenience wrapper for the bundled viewer contract. */
export async function internalPlantreeRowsV1Alpha2OrThrow(
  plan: PlanInput,
  format: Format = "CURRENT",
  config: InternalPlantreeConfigV1Alpha2 = {},
) {
  const result = await internalPlantreeRowsV1Alpha2(plan, format, config);
  if ("error" in result) {
    throw new Error(result.error);
  }
  return result.rows;
}

export { getBrowserWasm } from "./wasm-browser.js";
export { normalizePlanInput, parsePlanText } from "./input-browser.js";
export type * from "./types.js";
