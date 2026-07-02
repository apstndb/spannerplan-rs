"""Parse CLI flags and environment for Spanner client examples."""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from pathlib import Path

_VALID_MODES = frozenset({"PLAN", "PROFILE"})
_DEFAULT_QUERY_FILE = Path(__file__).resolve().parents[1] / "query.sql"


@dataclass(frozen=True)
class CliOptions:
    query_mode: str
    project: str
    instance: str
    database: str
    sql: str


def _usage() -> str:
    return (
        "usage: analyze_and_render.py [options]\n"
        "  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n"
        "  --project PROJECT           GCP project id\n"
        "  --instance INSTANCE         Spanner instance id\n"
        "  --database DATABASE         Spanner database id\n"
        "  --query SQL                 SQL text (overrides --query-file)\n"
        "  --query-file PATH           SQL file (default: ../query.sql)\n"
        "\n"
        "Environment (when flags omitted):\n"
        "  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n"
        "  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE"
    )


def _env_or_none(name: str) -> str | None:
    value = os.environ.get(name)
    if value is None or not value.strip():
        return None
    return value.strip()


def _load_sql(query: str | None, query_file: str | None) -> str:
    if query is not None:
        return query.strip()
    path = Path(query_file) if query_file else _DEFAULT_QUERY_FILE
    return path.read_text(encoding="utf-8").strip()


def parse_cli_options(argv: list[str] | None = None) -> CliOptions:
    args = list(argv if argv is not None else sys.argv[1:])
    query_mode = (_env_or_none("SPANNER_QUERY_MODE") or "PLAN").upper()
    project = _env_or_none("SPANNER_PROJECT_ID")
    instance = _env_or_none("SPANNER_INSTANCE_ID")
    database = _env_or_none("SPANNER_DATABASE_ID")
    query = _env_or_none("SPANNER_QUERY")
    query_file = _env_or_none("SPANNER_QUERY_FILE")

    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ("-h", "--help"):
            print(_usage(), file=sys.stderr)
            raise SystemExit(0)
        if arg == "--query-mode" and i + 1 < len(args):
            query_mode = args[i + 1].upper()
            i += 2
            continue
        if arg == "--project" and i + 1 < len(args):
            project = args[i + 1]
            i += 2
            continue
        if arg == "--instance" and i + 1 < len(args):
            instance = args[i + 1]
            i += 2
            continue
        if arg == "--database" and i + 1 < len(args):
            database = args[i + 1]
            i += 2
            continue
        if arg == "--query" and i + 1 < len(args):
            query = args[i + 1]
            i += 2
            continue
        if arg == "--query-file" and i + 1 < len(args):
            query_file = args[i + 1]
            i += 2
            continue
        raise SystemExit(f"unknown argument: {arg}")

    if query_mode not in _VALID_MODES:
        raise SystemExit(f"query mode must be PLAN or PROFILE, got: {query_mode}")
    if not project:
        raise SystemExit("missing required value: set --project or SPANNER_PROJECT_ID")
    if not instance:
        raise SystemExit("missing required value: set --instance or SPANNER_INSTANCE_ID")
    if not database:
        raise SystemExit("missing required value: set --database or SPANNER_DATABASE_ID")

    return CliOptions(
        query_mode=query_mode,
        project=project,
        instance=instance,
        database=database,
        sql=_load_sql(query, query_file),
    )
