import type {
  Format,
  PlanInput,
  RenderConfig,
  RenderMode,
  RenderResponse,
} from "./types.js";
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

export { getBrowserWasm } from "./wasm-browser.js";
export { normalizePlanInput } from "./input-browser.js";
export type * from "./types.js";
