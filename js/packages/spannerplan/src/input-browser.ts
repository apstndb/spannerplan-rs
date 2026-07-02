import type { PlanInput } from "./types.js";

/** Browser-safe plan normalization (YAML/JSON text, object, or wire bytes). */
export function normalizePlanInput(plan: PlanInput): string | Uint8Array {
  if (plan instanceof Uint8Array) {
    return plan;
  }
  if (typeof plan === "string") {
    return plan;
  }
  return JSON.stringify(plan);
}
