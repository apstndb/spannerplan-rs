import { parse as parseYaml } from "yaml";

import type { PlanInput } from "./types.js";

/**
 * Parse YAML or JSON plan text into a plain object for the slim WASM core.
 *
 * Browser builds omit `serde_yaml_ng`; the host parses text before calling WASM.
 */
export function parsePlanText(text: string): Record<string, unknown> {
  const trimmed = text.trim();
  if (!trimmed) {
    throw new Error("empty plan input");
  }

  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        return parsed as Record<string, unknown>;
      }
    } catch {
      // fall through to YAML
    }
  }

  const parsed: unknown = parseYaml(text);
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("plan input must be a YAML/JSON object");
  }
  return parsed as Record<string, unknown>;
}

/** Browser-safe plan normalization (host YAML/JSON, object, or wire bytes). */
export function normalizePlanInput(
  plan: PlanInput,
): Record<string, unknown> | Uint8Array {
  if (plan instanceof Uint8Array) {
    return plan;
  }
  if (typeof plan === "string") {
    return parsePlanText(plan);
  }
  return plan;
}
