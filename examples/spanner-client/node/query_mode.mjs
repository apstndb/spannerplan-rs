/** Parse Spanner execute-sql query mode (PLAN / PROFILE) for examples. */

const VALID = new Set(["PLAN", "PROFILE"]);

export function parseQueryMode(argv = process.argv.slice(2)) {
  let mode = (process.env.SPANNER_QUERY_MODE ?? "PLAN").toUpperCase();
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--query-mode" && i + 1 < argv.length) {
      mode = argv[i + 1].toUpperCase();
      i += 1;
      continue;
    }
    if (argv[i] === "-h" || argv[i] === "--help") {
      console.error(
        "usage: analyze_and_render.mjs [--query-mode PLAN|PROFILE]\n" +
          "  SPANNER_QUERY_MODE  same as --query-mode (default: PLAN)",
      );
      process.exit(0);
    }
    throw new Error(`unknown argument: ${argv[i]}`);
  }
  if (!VALID.has(mode)) {
    throw new Error(`query mode must be PLAN or PROFILE, got: ${mode}`);
  }
  return mode;
}

export function spannerQueryMode(mode, protos) {
  return mode === "PROFILE"
    ? protos.google.spanner.v1.ExecuteSqlRequest.QueryMode.PROFILE
    : protos.google.spanner.v1.ExecuteSqlRequest.QueryMode.PLAN;
}

export function renderModeFor(mode) {
  return mode === "PROFILE" ? "PROFILE" : "PLAN";
}
