#!/usr/bin/env python3
"""Fetch a QueryPlan via google-cloud-spanner and render with spannerplan FFI."""

from __future__ import annotations

from google.cloud import spanner

from cli_options import CliOptions, parse_cli_options
from query_mode import render_mode_for, spanner_execute_query_mode
from spanner_adapter import render_query_plan


def fetch_query_plan(opts: CliOptions):
    client = spanner.Client(project=opts.project)
    database = client.instance(opts.instance).database(opts.database)

    with database.snapshot() as snapshot:
        result_set = snapshot.execute_sql(
            opts.sql,
            query_mode=spanner_execute_query_mode(opts.query_mode),
        )
        for _ in result_set:
            pass
        stats = result_set.stats
        if stats is None or stats.query_plan is None:
            raise RuntimeError("QueryPlan missing from ResultSetStats")
        return stats.query_plan


def main() -> int:
    opts = parse_cli_options()
    plan = fetch_query_plan(opts)
    print(render_query_plan(plan, mode=render_mode_for(opts.query_mode)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
