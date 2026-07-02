/** Parse CLI flags and environment for Spanner client examples. */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VALID_MODES = new Set(["PLAN", "PROFILE"]);
const DEFAULT_QUERY_FILE = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "query.sql",
);

function envOrNull(name) {
  const value = process.env[name];
  if (!value?.trim()) {
    return null;
  }
  return value.trim();
}

function usage() {
  return (
    "usage: analyze_and_render.mjs [options]\n" +
    "  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n" +
    "  --project PROJECT           GCP project id\n" +
    "  --instance INSTANCE         Spanner instance id\n" +
    "  --database DATABASE         Spanner database id\n" +
    "  --query SQL                 SQL text (overrides --query-file)\n" +
    "  --query-file PATH           SQL file (default: ../query.sql)\n" +
    "\n" +
    "Environment (when flags omitted):\n" +
    "  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n" +
    "  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE"
  );
}

function loadSql(query, queryFile) {
  if (query != null) {
    return query.trim();
  }
  const path = queryFile ?? DEFAULT_QUERY_FILE;
  return readFileSync(path, "utf8").trim();
}

export function parseCliOptions(argv = process.argv.slice(2)) {
  let queryMode = (envOrNull("SPANNER_QUERY_MODE") ?? "PLAN").toUpperCase();
  let project = envOrNull("SPANNER_PROJECT_ID");
  let instance = envOrNull("SPANNER_INSTANCE_ID");
  let database = envOrNull("SPANNER_DATABASE_ID");
  let query = envOrNull("SPANNER_QUERY");
  let queryFile = envOrNull("SPANNER_QUERY_FILE");

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "-h" || arg === "--help") {
      console.error(usage());
      process.exit(0);
    }
    if (arg === "--query-mode" && i + 1 < argv.length) {
      queryMode = argv[++i].toUpperCase();
      continue;
    }
    if (arg === "--project" && i + 1 < argv.length) {
      project = argv[++i];
      continue;
    }
    if (arg === "--instance" && i + 1 < argv.length) {
      instance = argv[++i];
      continue;
    }
    if (arg === "--database" && i + 1 < argv.length) {
      database = argv[++i];
      continue;
    }
    if (arg === "--query" && i + 1 < argv.length) {
      query = argv[++i];
      continue;
    }
    if (arg === "--query-file" && i + 1 < argv.length) {
      queryFile = argv[++i];
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }

  if (!VALID_MODES.has(queryMode)) {
    throw new Error(`query mode must be PLAN or PROFILE, got: ${queryMode}`);
  }
  if (!project) {
    throw new Error("missing required value: set --project or SPANNER_PROJECT_ID");
  }
  if (!instance) {
    throw new Error("missing required value: set --instance or SPANNER_INSTANCE_ID");
  }
  if (!database) {
    throw new Error("missing required value: set --database or SPANNER_DATABASE_ID");
  }

  return {
    queryMode,
    project,
    instance,
    database,
    sql: loadSql(query, queryFile),
  };
}
