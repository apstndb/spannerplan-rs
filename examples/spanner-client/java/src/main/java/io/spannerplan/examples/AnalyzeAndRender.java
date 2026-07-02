package io.spannerplan.examples;

import com.google.cloud.spanner.DatabaseClient;
import com.google.cloud.spanner.DatabaseId;
import com.google.cloud.spanner.ReadContext;
import com.google.cloud.spanner.ResultSet;
import com.google.cloud.spanner.Spanner;
import com.google.cloud.spanner.SpannerOptions;
import com.google.cloud.spanner.Statement;
import com.google.spanner.v1.QueryPlan;

/** Fetch QueryPlan via google-cloud-spanner and render with spannerplan FFI. */
public final class AnalyzeAndRender {
  private AnalyzeAndRender() {}

  public static void main(String[] args) throws Exception {
    CliOptions opts = CliOptions.parse(args);

    SpannerOptions options = SpannerOptions.newBuilder().setProjectId(opts.project).build();
    try (Spanner spanner = options.getService()) {
      DatabaseClient client =
          spanner.getDatabaseClient(DatabaseId.of(opts.project, opts.instance, opts.database));
      try (ReadContext readContext = client.singleUse()) {
        try (ResultSet resultSet =
            readContext.analyzeQuery(
                Statement.of(opts.sql), QueryModeOption.toAnalyzeMode(opts.queryMode))) {
          while (resultSet.next()) {
            // Discard row data; PROFILE may return rows we do not need.
          }
          QueryPlan plan = resultSet.getStats().getQueryPlan();
          if (plan == null) {
            throw new IllegalStateException("QueryPlan missing from ResultSetStats");
          }
          System.out.println(
              SpannerAdapter.renderQueryPlan(plan, QueryModeOption.renderMode(opts.queryMode)));
        }
      }
    }
  }
}
