import type { BytesInput, PlanInput } from "./types.js";

/**
 * Normalize plan input for the WASM reference renderer.
 *
 * YAML and JSON text pass through to WASM (`serde_yaml_ng` in the Rust std
 * layer). Objects are serialized to JSON; protobuf wire bytes pass through.
 */
export function normalizePlanInput(plan: PlanInput): string | Uint8Array {
  if (plan instanceof Uint8Array) {
    return plan;
  }
  if (typeof plan === "string") {
    return plan;
  }
  return JSON.stringify(plan);
}

/**
 * Read stdin-style bytes (YAML or JSON) for the `rendertree` CLI WASM entry.
 */
export function normalizeStdinBytes(input: BytesInput): Uint8Array {
  if (typeof input === "string") {
    return new TextEncoder().encode(input);
  }
  if (input instanceof Uint8Array) {
    return input;
  }
  return new Uint8Array(input);
}
