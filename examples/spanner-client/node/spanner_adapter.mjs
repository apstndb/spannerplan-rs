/** Thin adapter: Spanner client QueryPlan → spannerplan wire bytes. */
import { protos } from "@google-cloud/spanner";

export function queryPlanToWire(queryPlan) {
  const message = protos.google.spanner.v1.QueryPlan.fromObject(queryPlan);
  return protos.google.spanner.v1.QueryPlan.encode(message).finish();
}
