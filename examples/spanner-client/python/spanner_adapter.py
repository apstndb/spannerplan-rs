"""Thin adapter: Spanner client QueryPlan → spannerplan wire bytes."""

from __future__ import annotations

from typing import Any


def query_plan_to_wire(query_plan: Any) -> bytes:
    """Serialize a google-cloud-spanner QueryPlan proto to wire bytes."""
    pb = getattr(query_plan, "_pb", query_plan)
    return pb.SerializeToString()


def render_query_plan(
    query_plan: Any,
    *,
    mode: str = "PLAN",
    format: str = "CURRENT",
) -> str:
    """Render a client-library QueryPlan without JSON/YAML."""
    from spannerplan import render_tree_table_wire

    return render_tree_table_wire(query_plan_to_wire(query_plan), mode=mode, format=format)
