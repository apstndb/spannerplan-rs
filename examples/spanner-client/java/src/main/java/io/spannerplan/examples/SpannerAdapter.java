package io.spannerplan.examples;

import com.google.spanner.v1.QueryPlan;
import io.spannerplan.Spannerplan;

/** Thin adapter: Spanner client QueryPlan → spannerplan wire render. */
public final class SpannerAdapter {
  private SpannerAdapter() {}

  public static String renderQueryPlan(QueryPlan plan, String renderMode) {
    return Spannerplan.renderTreeTableWire(plan.toByteArray(), renderMode, "CURRENT", null);
  }
}
