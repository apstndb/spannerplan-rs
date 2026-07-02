"""Parse Spanner execute-sql query mode (PLAN / PROFILE) for examples."""

from __future__ import annotations

import os
import sys

from google.cloud.spanner_v1 import ExecuteSqlRequest

_VALID = frozenset({"PLAN", "PROFILE"})


def parse_query_mode(argv: list[str] | None = None) -> str:
    """Return `PLAN` or `PROFILE` from `--query-mode` or `SPANNER_QUERY_MODE`."""
    args = list(argv if argv is not None else sys.argv[1:])
    mode = os.environ.get("SPANNER_QUERY_MODE", "PLAN").upper()
    i = 0
    while i < len(args):
        if args[i] == "--query-mode" and i + 1 < len(args):
            mode = args[i + 1].upper()
            i += 2
            continue
        if args[i] in ("-h", "--help"):
            print(
                "usage: analyze_and_render.py [--query-mode PLAN|PROFILE]\n"
                "  SPANNER_QUERY_MODE  same as --query-mode (default: PLAN)",
                file=sys.stderr,
            )
            raise SystemExit(0)
        raise SystemExit(f"unknown argument: {args[i]}")
    if mode not in _VALID:
        raise SystemExit(f"query mode must be PLAN or PROFILE, got: {mode}")
    return mode


def spanner_execute_query_mode(mode: str) -> ExecuteSqlRequest.QueryMode:
    if mode == "PROFILE":
        return ExecuteSqlRequest.QueryMode.PROFILE
    return ExecuteSqlRequest.QueryMode.PLAN


def render_mode_for(mode: str) -> str:
    """Map Spanner query mode to spannerplan render mode."""
    return "PROFILE" if mode == "PROFILE" else "PLAN"
