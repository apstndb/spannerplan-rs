package io.spannerplan.examples;

import java.nio.file.Files;
import java.nio.file.Path;

/** CLI flags and environment for Spanner client examples. */
public final class CliOptions {
  public final String queryMode;
  public final String project;
  public final String instance;
  public final String database;
  public final String sql;

  private CliOptions(String queryMode, String project, String instance, String database, String sql) {
    this.queryMode = queryMode;
    this.project = project;
    this.instance = instance;
    this.database = database;
    this.sql = sql;
  }

  public static CliOptions parse(String[] args) throws Exception {
    String queryMode = envOrDefault("SPANNER_QUERY_MODE", "PLAN").toUpperCase();
    String project = envOrNull("SPANNER_PROJECT_ID");
    String instance = envOrNull("SPANNER_INSTANCE_ID");
    String database = envOrNull("SPANNER_DATABASE_ID");
    String query = envOrNull("SPANNER_QUERY");
    String queryFile = envOrNull("SPANNER_QUERY_FILE");

    for (int i = 0; i < args.length; i++) {
      String arg = args[i];
      if ("-h".equals(arg) || "--help".equals(arg)) {
        printUsage();
        System.exit(0);
      }
      if ("--query-mode".equals(arg) && i + 1 < args.length) {
        queryMode = args[++i].toUpperCase();
        continue;
      }
      if ("--project".equals(arg) && i + 1 < args.length) {
        project = args[++i];
        continue;
      }
      if ("--instance".equals(arg) && i + 1 < args.length) {
        instance = args[++i];
        continue;
      }
      if ("--database".equals(arg) && i + 1 < args.length) {
        database = args[++i];
        continue;
      }
      if ("--query".equals(arg) && i + 1 < args.length) {
        query = args[++i];
        continue;
      }
      if ("--query-file".equals(arg) && i + 1 < args.length) {
        queryFile = args[++i];
        continue;
      }
      throw new IllegalArgumentException("unknown argument: " + arg);
    }

    if (!"PLAN".equals(queryMode) && !"PROFILE".equals(queryMode)) {
      throw new IllegalArgumentException("query mode must be PLAN or PROFILE, got: " + queryMode);
    }
    requireValue("project", "SPANNER_PROJECT_ID", project);
    requireValue("instance", "SPANNER_INSTANCE_ID", instance);
    requireValue("database", "SPANNER_DATABASE_ID", database);

    String sql = loadSql(query, queryFile);
    return new CliOptions(queryMode, project, instance, database, sql);
  }

  private static void requireValue(String flag, String env, String value) {
    if (value == null || value.isBlank()) {
      throw new IllegalStateException(
          "missing required value: set --" + flag + " or " + env);
    }
  }

  private static String loadSql(String query, String queryFile) throws Exception {
    if (query != null && !query.isBlank()) {
      return query.trim();
    }
    Path path =
        queryFile != null && !queryFile.isBlank()
            ? Path.of(queryFile)
            : Path.of("..", "query.sql");
    return Files.readString(path).trim();
  }

  private static String envOrNull(String name) {
    String value = System.getenv(name);
    if (value == null || value.isBlank()) {
      return null;
    }
    return value.trim();
  }

  private static String envOrDefault(String name, String defaultValue) {
    String value = envOrNull(name);
    return value != null ? value : defaultValue;
  }

  private static void printUsage() {
    System.err.println(
        "usage: AnalyzeAndRender [options]\n"
            + "  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n"
            + "  --project PROJECT           GCP project id\n"
            + "  --instance INSTANCE         Spanner instance id\n"
            + "  --database DATABASE         Spanner database id\n"
            + "  --query SQL                 SQL text (overrides --query-file)\n"
            + "  --query-file PATH           SQL file (default: ../query.sql)\n"
            + "\n"
            + "Environment (when flags omitted):\n"
            + "  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n"
            + "  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE");
  }
}
