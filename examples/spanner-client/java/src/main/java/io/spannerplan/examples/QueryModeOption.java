package io.spannerplan.examples;

import com.google.cloud.spanner.ReadContext;

/** Spanner execute-sql query mode (PLAN / PROFILE) for examples. */
public final class QueryModeOption {
  private QueryModeOption() {}

  public static String parse(String[] args) {
    String mode = System.getenv().getOrDefault("SPANNER_QUERY_MODE", "PLAN").toUpperCase();
    for (int i = 0; i < args.length; i++) {
      if ("--query-mode".equals(args[i]) && i + 1 < args.length) {
        mode = args[i + 1].toUpperCase();
        break;
      }
      if ("-h".equals(args[i]) || "--help".equals(args[i])) {
        System.err.println(
            "usage: AnalyzeAndRender [--query-mode PLAN|PROFILE]\n"
                + "  SPANNER_QUERY_MODE  same as --query-mode (default: PLAN)");
        System.exit(0);
      }
      throw new IllegalArgumentException("unknown argument: " + args[i]);
    }
    if (!"PLAN".equals(mode) && !"PROFILE".equals(mode)) {
      throw new IllegalArgumentException("query mode must be PLAN or PROFILE, got: " + mode);
    }
    return mode;
  }

  public static ReadContext.QueryAnalyzeMode toAnalyzeMode(String mode) {
    return "PROFILE".equals(mode)
        ? ReadContext.QueryAnalyzeMode.PROFILE
        : ReadContext.QueryAnalyzeMode.PLAN;
  }

  public static String renderMode(String mode) {
    return "PROFILE".equals(mode) ? "PROFILE" : "PLAN";
  }
}
