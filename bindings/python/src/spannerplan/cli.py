"""Minimal `rendertree` CLI: stdin plan YAML/JSON -> ASCII table on stdout."""

from __future__ import annotations

import argparse
import sys
from typing import Any, Mapping, Sequence

from spannerplan import RenderError, render_tree_table_json

USAGE_EXIT = 2


class UsageError(Exception):
    """Flag or usage validation failure (exit code 2)."""


_PRINT_PRESETS: dict[str, list[str] | None] = {
    "basic": None,
    "enhanced": ["predicates", "ordering", "aggregate"],
    "full": ["full"],
    "none": [],
}

_PRINT_SECTIONS = frozenset({"predicates", "ordering", "aggregate", "typed", "full"})


def parse_print(value: str) -> list[str] | None:
    """Map -print to RenderConfig.printSections (None = Rust default)."""
    trimmed = value.strip()
    if not trimmed:
        return []

    if "," not in trimmed:
        key = trimmed.casefold()
        if key in _PRINT_PRESETS:
            return _PRINT_PRESETS[key]
        if key in _PRINT_SECTIONS:
            return [key]
        raise UsageError(f"unknown print preset or section: {trimmed!r}")

    sections: list[str] = []
    for raw in trimmed.split(","):
        token = raw.strip().casefold()
        if not token:
            raise UsageError("print section must not be empty")
        if token in _PRINT_PRESETS:
            raise UsageError(f"print preset {raw.strip()!r} cannot be combined with section list")
        if token not in _PRINT_SECTIONS:
            raise UsageError(f"unknown print section: {raw.strip()!r}")
        if token in sections:
            raise UsageError(f"duplicate print section: {token}")
        sections.append(token)

    if len(sections) > 1 and any(s in {"typed", "full"} for s in sections):
        for section in sections:
            if section in {"typed", "full"}:
                raise UsageError(
                    f"print section {section!r} cannot be combined with other sections"
                )

    return sections


def build_config(
    *,
    print_value: str,
    wrap_width: int,
) -> dict[str, Any] | None:
    config: dict[str, Any] = {}
    print_sections = parse_print(print_value)
    if print_sections is not None:
        config["printSections"] = print_sections
    if wrap_width != 0:
        config["wrapWidth"] = wrap_width
    return config or None


def print_usage(stream: Any | None = None) -> None:
    if stream is None:
        stream = sys.stderr
    print("Usage of rendertree:", file=stream)
    print(
        "  -compact\n"
        "    \tEnable compact format\n"
        "  -h\n"
        "    \tShow this help message\n"
        "  -mode string\n"
        '    \tPROFILE, PLAN, AUTO (ignore case) (default "AUTO")\n'
        "  -print string\n"
        '    \tAppendix preset (basic, enhanced, full, none) or comma-separated '
        'sections (default "basic")\n'
        "  -wrap-width int\n"
        "    \tWrap Operator column at this width; 0 disables wrapping (default 0)",
        file=stream,
    )


def run(args: Sequence[str], stdin: bytes) -> int:
    """Parse flags, render stdin, write stdout/stderr. Returns process exit code."""
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("-h", "-help", "--help", action="store_true", dest="help")
    parser.add_argument("-mode", "--mode", default="AUTO")
    parser.add_argument("-print", "--print", default="basic", dest="print_")
    parser.add_argument("-compact", "--compact", action="store_true")
    parser.add_argument("-wrap-width", "--wrap-width", type=int, default=0, dest="wrap_width")

    try:
        known, unknown = parser.parse_known_args(list(args))
    except argparse.ArgumentError as exc:
        print(exc, file=sys.stderr)
        print_usage()
        return USAGE_EXIT

    if unknown:
        flag = unknown[0]
        print(f"flag provided but not defined: {flag}", file=sys.stderr)
        print_usage()
        return USAGE_EXIT

    if known.help:
        print_usage()
        return 0

    mode = known.mode
    if mode.casefold() not in {"auto", "plan", "profile"}:
        print(
            f"Invalid value for -mode flag: invalid input: {mode}. "
            "Must be one of AUTO, PLAN, PROFILE (case-insensitive).",
            file=sys.stderr,
        )
        print_usage()
        return USAGE_EXIT

    try:
        config = build_config(print_value=known.print_, wrap_width=known.wrap_width)
    except UsageError as exc:
        print(f"Invalid value for -print flag: {exc}", file=sys.stderr)
        print_usage()
        return USAGE_EXIT

    if known.wrap_width < 0:
        print(
            f"Invalid value for -wrap-width flag: wrapWidth cannot be negative: {known.wrap_width}",
            file=sys.stderr,
        )
        print_usage()
        return USAGE_EXIT

    format_name = "COMPACT" if known.compact else "CURRENT"

    try:
        output = render_tree_table_json(
            stdin,
            mode=mode.upper(),
            format=format_name,
            config=config,
        )
    except RenderError as exc:
        print(exc, file=sys.stderr)
        return 1

    sys.stdout.write(output)
    return 0


def main() -> None:
    raise SystemExit(run(sys.argv[1:], sys.stdin.buffer.read()))


if __name__ == "__main__":
    main()
