#!/usr/bin/env python3
"""Smoke test for spannerplan-ffi via ctypes."""

from __future__ import annotations

import ctypes
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LIB_NAME = {
    "darwin": "libspannerplan_ffi.dylib",
    "linux": "libspannerplan_ffi.so",
}.get(sys.platform)

if LIB_NAME is None:
    raise SystemExit(f"unsupported platform for smoke test: {sys.platform}")

lib_path = ROOT / "target" / "debug" / LIB_NAME
if not lib_path.exists():
    raise SystemExit(f"missing library at {lib_path}; run `cargo build -p spannerplan-ffi` first")

lib = ctypes.CDLL(str(lib_path))

lib.spannerplan_render_tree_table_json.argtypes = [
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_int),
]
lib.spannerplan_render_tree_table_json.restype = ctypes.c_void_p
lib.spannerplan_string_free.argtypes = [ctypes.c_void_p]
lib.spannerplan_string_free.restype = None

fixture = (ROOT / "testdata/reference/dca.yaml").read_bytes()
is_error = ctypes.c_int(0)
out = lib.spannerplan_render_tree_table_json(
    fixture,
    b"AUTO",
    b"CURRENT",
    None,
    ctypes.byref(is_error),
)
if not out:
    raise SystemExit("render returned NULL")
try:
    text = ctypes.string_at(out).decode("utf-8")
finally:
    lib.spannerplan_string_free(out)

if is_error.value != 0:
    raise SystemExit(f"render failed: {text}")
if "Distributed Cross Apply" not in text:
    raise SystemExit("unexpected render output")

nul_plan = (
    b'{"planNodes":[{"index":0,"kind":"RELATIONAL",'
    b'"displayName":"Scan\\u0000Injected"}]}'
)
is_error = ctypes.c_int(0)
out = lib.spannerplan_render_tree_table_json(
    nul_plan,
    b"AUTO",
    b"CURRENT",
    None,
    ctypes.byref(is_error),
)
if not out:
    raise SystemExit("interior-NUL render returned NULL")
try:
    text = ctypes.string_at(out).decode("utf-8")
finally:
    lib.spannerplan_string_free(out)

if is_error.value != 1:
    raise SystemExit(f"interior-NUL render returned status {is_error.value}, expected 1")
if text != "render result contains an interior NUL byte":
    raise SystemExit(f"unexpected interior-NUL diagnostic: {text!r}")

print("ctypes smoke test ok")
