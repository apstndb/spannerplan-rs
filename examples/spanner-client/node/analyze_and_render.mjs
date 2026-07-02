import { Spanner, protos } from "@google-cloud/spanner";
import { renderTreeTableWire } from "@spannerplan/core";
import { parseCliOptions } from "./cli_options.mjs";
import {
  renderModeFor,
  spannerQueryMode,
} from "./query_mode.mjs";
import { queryPlanToWire } from "./spanner_adapter.mjs";

const opts = parseCliOptions();

const spanner = new Spanner({ projectId: opts.project });
const database = spanner.instance(opts.instance).database(opts.database);

const [rows, stats] = await database.run({
  sql: opts.sql,
  queryMode: spannerQueryMode(opts.queryMode, protos),
});

for (const _row of rows) {
  // Discard row data; PROFILE executes the query.
}

const plan = stats?.queryPlan;
if (!plan) {
  throw new Error("QueryPlan missing from ResultSetStats");
}

const result = renderTreeTableWire(
  queryPlanToWire(plan),
  renderModeFor(opts.queryMode),
  "CURRENT",
);
if ("error" in result) {
  throw new Error(result.error);
}
process.stdout.write(result.output);
